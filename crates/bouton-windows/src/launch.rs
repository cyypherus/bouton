pub fn launch_wsl_client(device: &str, server: &str, sudo: bool) -> Result<(), String> {
    let (raw_host, port) = match server.rsplit_once(':') {
        Some((h, p)) => (h, p),
        None => return Err(format!("invalid server address: {server}")),
    };
    let host = if raw_host.is_empty() || raw_host == "0.0.0.0" {
        detect_windows_host()?
    } else {
        raw_host.to_string()
    };
    let host_port = format!("{host}:{port}");
    let device_q = sh_quote(device);
    let server_q = sh_quote(&host_port);
    let bin = if sudo {
        "sudo \"$(command -v bouton-linux)\""
    } else {
        "bouton-linux"
    };
    let script = format!(
        "echo \"[Windows host: {host}]\"; \
         {bin} --run {device_q} {server_q}; \
         echo; \
         echo \"[bouton-linux exited: $?] press Enter to close\"; \
         read"
    );
    spawn_console(&script)
}

#[cfg(target_os = "windows")]
fn detect_windows_host() -> Result<String, String> {
    use std::process::Command;
    let out = Command::new("wsl.exe")
        .args(["--", "sh", "-c", "ip route show | grep default"])
        .output()
        .map_err(|e| format!("wsl detect: {e}"))?;
    let line = String::from_utf8_lossy(&out.stdout);
    let ip = line.split_whitespace().nth(2).unwrap_or("").to_string();
    if ip.is_empty() {
        Err("could not detect Windows host IP from WSL. Set a real IP in the Server Address field.".into())
    } else {
        Ok(ip)
    }
}

#[cfg(not(target_os = "windows"))]
fn detect_windows_host() -> Result<String, String> {
    Err("WSL host detection is only supported on Windows".into())
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
