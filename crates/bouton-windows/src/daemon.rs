use bouton_core::control::GamepadControl;
use serde::Deserialize;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::oneshot;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonMsg {
    Device { path: String, name: String },
    Opened,
    PermissionDenied,
    OpenError { msg: String },
    SendError { msg: String },
    Button { control: GamepadControl, pressed: bool },
    Axis { control: GamepadControl, value: i32 },
}

#[derive(Debug, Clone)]
pub enum DaemonEvent {
    Msg(DaemonMsg),
    Stderr(String),
    Exited(Option<i32>),
}

type Callback = Arc<dyn Fn(DaemonEvent) + Send + Sync + 'static>;

fn sh_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn wsl_cmd(as_root: bool, args: &[&str]) -> Command {
    let mut script = String::from(". \"$HOME/.cargo/env\" 2>/dev/null; exec bouton-linux");
    for a in args {
        script.push(' ');
        script.push_str(&sh_quote(a));
    }
    let mut cmd = Command::new("wsl");
    if as_root {
        cmd.args(["-u", "root"]);
    }
    cmd.arg("--");
    cmd.args(["bash", "-lc", &script]);
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    cmd
}

#[derive(Debug, Clone)]
pub enum ListError {
    WslMissing(String),
    BoutonLinuxMissing,
    Failed { code: Option<i32>, stderr: String },
}

pub struct ListResult {
    pub devices: Vec<(String, String)>,
    pub error: Option<ListError>,
}

pub async fn list_devices(as_root: bool) -> ListResult {
    let mut cmd = wsl_cmd(as_root, &["--list"]);
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return ListResult {
                devices: Vec::new(),
                error: Some(ListError::WslMissing(e.to_string())),
            };
        }
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let mut out = Vec::new();
    if let Some(s) = stdout {
        let mut lines = BufReader::new(s).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(DaemonMsg::Device { path, name }) = serde_json::from_str(&line) {
                out.push((path, name));
            }
        }
    }

    let mut err_text = String::new();
    if let Some(s) = stderr {
        let mut lines = BufReader::new(s).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if !err_text.is_empty() {
                err_text.push('\n');
            }
            err_text.push_str(&line);
        }
    }

    let status = child.wait().await.ok();
    let code = status.as_ref().and_then(|s| s.code());
    let success = status.as_ref().is_some_and(|s| s.success());

    let error = if success {
        None
    } else if looks_like_missing_binary(&err_text) {
        Some(ListError::BoutonLinuxMissing)
    } else {
        Some(ListError::Failed {
            code,
            stderr: err_text,
        })
    };

    ListResult {
        devices: out,
        error,
    }
}

fn looks_like_missing_binary(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    lower.contains("command not found")
        || lower.contains("bouton-linux: not found")
        || lower.contains("no such file or directory")
        || lower.contains("not recognized")
}

pub struct DaemonHandle {
    kill: Option<oneshot::Sender<()>>,
}

impl DaemonHandle {
    pub fn stop(&mut self) {
        if let Some(k) = self.kill.take() {
            let _ = k.send(());
        }
    }
}

pub fn handle() -> (DaemonHandle, oneshot::Receiver<()>) {
    let (kill_tx, kill_rx) = oneshot::channel();
    (
        DaemonHandle {
            kill: Some(kill_tx),
        },
        kill_rx,
    )
}

pub async fn run(
    device: String,
    server: String,
    as_root: bool,
    kill_rx: oneshot::Receiver<()>,
    on_event: impl Fn(DaemonEvent) + Send + Sync + 'static,
) {
    let cb: Callback = Arc::new(on_event);
    let mut cmd = wsl_cmd(as_root, &["--run", &device, &server]);
    let mut child: Child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            cb(DaemonEvent::Msg(DaemonMsg::OpenError {
                msg: format!("spawn wsl: {e}"),
            }));
            cb(DaemonEvent::Exited(None));
            return;
        }
    };
    pump(&mut child, cb, kill_rx).await;
}

async fn pump(child: &mut Child, cb: Callback, mut kill_rx: oneshot::Receiver<()>) {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_task = {
        let cb = cb.clone();
        async move {
            let Some(s) = stdout else { return };
            let mut lines = BufReader::new(s).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                match serde_json::from_str::<DaemonMsg>(&line) {
                    Ok(msg) => cb(DaemonEvent::Msg(msg)),
                    Err(_) => cb(DaemonEvent::Stderr(line)),
                }
            }
        }
    };
    let stderr_task = {
        let cb = cb.clone();
        async move {
            let Some(s) = stderr else { return };
            let mut lines = BufReader::new(s).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                cb(DaemonEvent::Stderr(line));
            }
        }
    };

    tokio::pin!(stdout_task);
    tokio::pin!(stderr_task);

    loop {
        tokio::select! {
            _ = &mut stdout_task => break,
            _ = &mut stderr_task => {}
            _ = &mut kill_rx => {
                let _ = child.kill().await;
                break;
            }
        }
    }

    let code = child.wait().await.ok().and_then(|s| s.code());
    cb(DaemonEvent::Exited(code));
}
