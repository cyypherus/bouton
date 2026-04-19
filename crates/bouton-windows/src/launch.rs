pub fn launch_wsl_client(device: &str, server: &str, sudo: bool) -> Result<(), String> {
    let device_q = sh_quote(device);
    let server_q = sh_quote(server);
    let prefix = if sudo { "sudo " } else { "" };
    let script = format!(
        "{prefix}bouton-linux --run {device_q} {server_q}; \
         echo; \
         echo \"[bouton-linux exited: $?] press Enter to close\"; \
         read"
    );
    spawn_console(&script)
}

#[cfg(target_os = "windows")]
fn spawn_console(script: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
    let mut cmd = Command::new("wsl.exe");
    cmd.args(["--", "bash", "-lic"]);
    cmd.arg(script);
    cmd.creation_flags(CREATE_NEW_CONSOLE);
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("spawn wsl.exe: {e}"))
}

#[cfg(not(target_os = "windows"))]
fn spawn_console(_script: &str) -> Result<(), String> {
    Err("WSL launch is only supported on Windows".to_string())
}

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
