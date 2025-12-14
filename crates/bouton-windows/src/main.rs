mod config;
mod key_injector;
mod keycode;
mod socket_server;

use config::Config;
use socket_server::SocketServer;
use std::env;
use std::path::PathBuf;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    let config_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("bouton.toml")
    };

    if !config_path.exists() {
        eprintln!("Config file not found: {}", config_path.display());
        eprintln!("Creating default config at {}", config_path.display());
        std::fs::write(&config_path, Config::default_toml())?;
        println!("Edit {} and run again", config_path.display());
        println!("\nPress Enter to exit...");
        let _ = std::io::stdin().read_line(&mut String::new());
        return Ok(());
    }

    let config = match Config::load(&config_path) {
        Ok(cfg) => {
            println!("Loaded config from {}", config_path.display());
            cfg
        }
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            println!("\nPress Enter to exit...");
            let _ = std::io::stdin().read_line(&mut String::new());
            return Err(e);
        }
    };

    // Build button code to key code mapping
    let button_map: HashMap<u16, u32> = config
        .keys
        .buttons
        .iter()
        .filter_map(|(button_name, key_code_enum)| {
            let button_code = match button_name.as_str() {
                "Square" => Some(0x130),
                "Cross" => Some(0x131),
                "Circle" => Some(0x132),
                "Triangle" => Some(0x133),
                "L1" => Some(0x134),
                "R1" => Some(0x135),
                "L2" => Some(0x136),
                "R2" => Some(0x137),
                "Select" => Some(0x138),
                "Start" => Some(0x139),
                "L3" => Some(0x13A),
                "R3" => Some(0x13B),
                "Touch" => Some(0x13D),
                "Aux1" => Some(0x13C),
                "Aux2" => Some(0x13E),
                _ => None,
            };
            button_code.map(|code| (code, key_code_enum.code()))
        })
        .collect();

    println!("Mapped {} buttons from config", button_map.len());

    // Build joystick configs
    let mut joystick_map: HashMap<bouton_core::control::GamepadControl, config::JoystickCodeConfig> = HashMap::new();
    for (stick_name, stick_config) in config.keys.joysticks.iter() {
        let control = match stick_name.as_str() {
            "LeftStick" => Some(bouton_core::control::GamepadControl::LeftStickX),
            "RightStick" => Some(bouton_core::control::GamepadControl::RightStickX),
            _ => None,
        };
        
        if let Some(control) = control {
            joystick_map.insert(
                control,
                config::JoystickCodeConfig {
                    deadzone: stick_config.deadzone.unwrap_or(20),
                    up: stick_config.up.code(),
                    down: stick_config.down.code(),
                    left: stick_config.left.code(),
                    right: stick_config.right.code(),
                },
            );
        }
    }

    println!("Mapped {} joysticks from config", joystick_map.len());

    // Build trigger configs
    let trigger_map: HashMap<u16, config::TriggerCodeConfig> = config
        .keys
        .triggers
        .iter()
        .filter_map(|(trigger_name, trigger_config)| {
            let axis_code = match trigger_name.as_str() {
                "L2" => Some(0x03),
                "R2" => Some(0x04),
                _ => None,
            };
            
            axis_code.map(|code| (
                code,
                config::TriggerCodeConfig {
                    key: trigger_config.key.code(),
                    threshold: trigger_config.threshold.unwrap_or(127),
                }
            ))
        })
        .collect();

    println!("Mapped {} triggers from config", trigger_map.len());

    // Build D-Pad config
    let dpad_config: Option<config::DPadCodeConfig> = config
        .keys
        .dpad
        .get("DPad")
        .map(|dpad| config::DPadCodeConfig {
            up: dpad.up.code(),
            down: dpad.down.code(),
            left: dpad.left.code(),
            right: dpad.right.code(),
        });

    if dpad_config.is_some() {
        println!("Mapped D-Pad from config");
    }

    let addr = format!("{}:{}", config.server.listen_addr, config.server.listen_port);
    let addr: std::net::SocketAddr = addr.parse()?;
    
    let server = SocketServer::bind(addr, button_map, joystick_map, trigger_map, dpad_config).await?;
    
    println!("Bouton server listening on {}", addr);
    server.run().await?;
    
    Ok(())
}
