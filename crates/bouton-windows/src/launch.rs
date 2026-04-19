pub fn launch_wsl_client(device: &str, server: &str, sudo: bool) -> Result<(), String> {
    let (host, port) = match server.rsplit_once(':') {
        Some((h, p)) => (h, p),
        None => return Err(format!("invalid server address: {server}")),
    };
    let host_q = sh_quote(host);
    let port_q = sh_quote(port);
    let device_q = sh_quote(device);
    let bin = if sudo {
        "sudo \"$(command -v bouton-linux)\""
    } else {
        "bouton-linux"
    };
    let script = format!(
        "host={host_q}; \
         port={port_q}; \
         if [ -z \"$host\" ] || [ \"$host\" = '0.0.0.0' ]; then \
           detected=\"$(ip route show default 2>/dev/null | awk '{{print $3; exit}}')\"; \
           if [ -z \"$detected\" ]; then \
             detected=\"$(cat /etc/resolv.conf 2>/dev/null | awk '/^nameserver/ {{print $2; exit}}')\"; \
           fi; \
           if [ -z \"$detected\" ]; then \
             echo \"could not auto-detect Windows host IP from WSL.\" >&2; \
             echo \"set a real IP in the Server Address field (not 0.0.0.0).\" >&2; \
             echo; \
             echo \"press Enter to close\"; \
             read; \
             exit 1; \
           fi; \
           host=\"$detected\"; \
           echo \"[auto-detected Windows host: $host]\"; \
         fi; \
         {bin} --run {device_q} \"$host:$port\"; \
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
