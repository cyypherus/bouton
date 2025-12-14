use std::io::{self, Write};
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::env;
use std::path::PathBuf;

fn main() {
    println!("=== Bouton Setup ===\n");

    // Step 1: Select usbipd device
    let busid = select_usbipd_device();

    // Step 2: Get WSL IP
    let wsl_ip = get_wsl_ip();

    // Step 3: Get deadzone
    let deadzone = get_deadzone();

    // Step 4: Get config file path
    let config_path = get_config_path();

    println!("\n=== Configuration ===");
    println!("USB Device: {}", busid);
    println!("WSL IP: {}", wsl_ip);
    println!("Deadzone: {}", deadzone);
    println!("Config File: {}", config_path);

    // Step 5: Attach USB device
    println!("\n=== Attaching USB Device ===");
    attach_usb_device(&busid);

    // Step 6: Get Linux event device
    println!("\n=== Finding Linux Event Device ===");
    let event_device = get_linux_event_device();

    // Step 7: Launch Windows server
    println!("\n=== Launching Bouton Server ===");
    launch_windows_server(&config_path);

    // Step 8: Launch Linux client
    println!("\n=== Launching Bouton Client ===");
    launch_linux_client(&event_device, &wsl_ip, deadzone);
}

fn select_usbipd_device() -> String {
    println!("Step 1: Select USB Device");
    println!("Running 'usbipd list'...\n");

    let output = Command::new("usbipd").arg("list").output();

    match output {
        Ok(result) => {
            if result.status.success() {
                let devices = String::from_utf8_lossy(&result.stdout);
                println!("{}", devices);
            } else {
                eprintln!("Could not run 'usbipd list'");
            }
        }
        Err(e) => {
            eprintln!("Error running usbipd: {}", e);
        }
    }

    print!("Enter the bus ID (e.g., 2-3): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn get_wsl_ip() -> String {
    println!("\nStep 2: Get Windows Host IP (from WSL perspective)");
    println!("Detecting Windows IP...\n");

    // Run 'ip route show' in WSL to find the default gateway (Windows host IP)
    let output = Command::new("wsl")
        .args(&["sh", "-c", "ip route show | grep default"])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                let line = String::from_utf8_lossy(&result.stdout);
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() >= 3 {
                    let ip = parts[2];
                    if !ip.is_empty() {
                        println!("Detected Windows IP: {}\n", ip);
                        return ip.to_string();
                    }
                }
            }
        }
        Err(_) => {}
    }

    // Fallback to manual entry
    println!("Could not auto-detect Windows IP");
    print!("Enter Windows IP (e.g., 172.21.192.1): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn get_deadzone() -> i32 {
    println!("\nStep 3: Joystick Deadzone");
    print!("Enter deadzone value (0-100, default 0): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().parse().unwrap_or(0)
}

fn get_config_path() -> String {
    println!("\nStep 4: Config File");
    print!("Enter config file path (default bouton.toml): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let path = input.trim().to_string();
    if path.is_empty() {
        "bouton.toml".to_string()
    } else {
        path
    }
}

fn attach_usb_device(busid: &str) {
    println!("Attaching USB device {}...\n", busid);

    let output = Command::new("usbipd")
        .args(&["attach", "--wsl", &format!("--busid={}", busid)])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                println!("✓ USB device attached\n");
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                if stderr.contains("already attached") {
                    println!("ℹ USB device already attached, continuing...\n");
                } else {
                    eprintln!("✗ Failed to attach USB device");
                    eprintln!("{}\n", stderr);
                }
            }
        }
        Err(e) => {
            eprintln!("✗ Error: {}\n", e);
        }
    }
}

fn get_linux_event_device() -> String {
    loop {
        println!("Step 5: Select Linux Event Device");
        println!("Running 'ls /dev/input/event*' on WSL...\n");

        let output = Command::new("wsl")
            .args(&["ls", "-1", "/dev/input/event*"])
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    let devices = String::from_utf8_lossy(&result.stdout);
                    let event_list: Vec<&str> = devices.lines().collect();

                    if !event_list.is_empty() {
                        println!("Found {} event device(s):", event_list.len());
                        for (i, dev) in event_list.iter().enumerate() {
                            println!("  [{}] {}", i + 1, dev);
                        }

                        print!("\nSelect device [1]: ");
                        io::stdout().flush().unwrap();

                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap();

                        let idx = input.trim().parse::<usize>().unwrap_or(1) - 1;
                        println!();
                        if idx < event_list.len() {
                            return event_list[idx].to_string();
                        } else {
                            return event_list[0].to_string();
                        }
                    }
                }
            }
            Err(_) => {}
        }

        println!("No event devices found.");
        println!("Make sure the gamepad is connected and USB device is attached.\n");

        print!("Retry? [Y/n]: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if input.trim().to_lowercase() == "n" {
            break;
        }
    }

    print!("Enter event device path manually (e.g., /dev/input/event0): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn launch_windows_server(config_path: &str) {
    println!("Launching bouton-windows.exe...\n");

    // Convert to absolute path if relative
    let config_path = std::path::Path::new(config_path);
    let config_path = if config_path.is_absolute() {
        config_path.to_string_lossy().to_string()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(config_path)
            .to_string_lossy()
            .to_string()
    };

    let status = Command::new("cmd")
        .args(&["/C", "start", "bouton-windows.exe", &config_path])
        .status();

    match status {
        Ok(result) => {
            if result.success() {
                println!("✓ Windows server launched\n");
                thread::sleep(Duration::from_millis(1000));
            } else {
                eprintln!("✗ Failed to launch Windows server\n");
            }
        }
        Err(e) => {
            eprintln!("✗ Error: {}\n", e);
        }
    }
}

fn launch_linux_client(event_device: &str, wsl_ip: &str, deadzone: i32) {
    println!("Launching Linux client via WSL...\n");
    println!("Device: {}", event_device);
    println!("Server: {}:8000", wsl_ip);
    println!("Deadzone: {}\n", deadzone);

    let mut args = vec![
        "sudo".to_string(),
        "bouton-linux".to_string(),
        event_device.to_string(),
        format!("{}:8000", wsl_ip),
    ];

    if deadzone > 0 {
        args.push(deadzone.to_string());
    }

    let output = Command::new("wsl").args(&args).status();

    match output {
        Ok(status) => {
            if status.success() {
                println!("\n✓ Linux client exited successfully");
            } else {
                eprintln!("\n✗ Linux client exited with error");
            }
        }
        Err(e) => {
            eprintln!("\n✗ Error: {}", e);
        }
    }
}
