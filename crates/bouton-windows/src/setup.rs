use std::process::Command;

#[derive(Debug, Clone)]
pub struct UsbDevice {
    pub busid: String,
    pub description: String,
    pub state: String,
}

pub fn list_devices() -> Result<Vec<UsbDevice>, String> {
    let output = Command::new("usbipd")
        .arg("list")
        .output()
        .map_err(|e| format!("usbipd not found: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();
    let mut in_connected = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Connected:") {
            in_connected = true;
            continue;
        }
        if trimmed.starts_with("Persisted:") {
            in_connected = false;
            continue;
        }
        if !in_connected || trimmed.is_empty() || trimmed.starts_with("BUSID") {
            continue;
        }
        let mut parts = trimmed.splitn(3, char::is_whitespace);
        let busid = parts.next().unwrap_or("").trim().to_string();
        let rest = parts.next().unwrap_or("").trim().to_string();
        let tail = parts.next().unwrap_or("").trim().to_string();
        if busid.is_empty() || !busid.contains('-') {
            continue;
        }
        let (description, state) = match tail.rsplit_once(char::is_whitespace) {
            Some((desc, st)) => (format!("{rest} {desc}").trim().to_string(), st.to_string()),
            None => (rest, tail),
        };
        devices.push(UsbDevice {
            busid,
            description,
            state,
        });
    }
    Ok(devices)
}

pub fn bind(busid: &str) -> Result<(), String> {
    let output = Command::new("usbipd")
        .args(["bind", "--busid", busid])
        .output()
        .map_err(|e| format!("usbipd bind: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&output.stderr);
    if err.contains("already shared") || err.contains("already bound") {
        return Ok(());
    }
    Err(err.into_owned())
}

pub fn attach(busid: &str) -> Result<(), String> {
    let output = Command::new("usbipd")
        .args(["attach", "--wsl", &format!("--busid={busid}")])
        .output()
        .map_err(|e| format!("usbipd attach: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&output.stderr);
    if err.contains("already attached") {
        return Ok(());
    }
    Err(err.into_owned())
}

pub fn detach(busid: &str) -> Result<(), String> {
    let output = Command::new("usbipd")
        .args(["detach", &format!("--busid={busid}")])
        .output()
        .map_err(|e| format!("usbipd detach: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}
