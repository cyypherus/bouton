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
        return Ok(());
    }

    let config = Config::load(&config_path)?;
    println!("Loaded config from {}", config_path.display());

    // Build button code to key code mapping
    let button_map: std::collections::HashMap<u16, u32> = config
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

    // Build axis code to config mapping
    let axis_map: HashMap<u16, config::AxisCodeConfig> = config
        .keys
        .axes
        .iter()
        .filter_map(|(axis_name, axis_config)| {
            let axis_code = match axis_name.as_str() {
                "LX" => Some(0x00),   // Left Stick X
                "LY" => Some(0x01),   // Left Stick Y
                "RX" => Some(0x02),   // Right Stick X
                "RY" => Some(0x05),   // Right Stick Y
                "L2" => Some(0x03),   // L2 Trigger
                "R2" => Some(0x04),   // R2 Trigger
                "DPadX" => Some(0x10), // D-Pad X
                "DPadY" => Some(0x11), // D-Pad Y
                _ => None,
            };
            
            // Convert KeyCode enums to codes
            let above_code = axis_config.above.map(|kc| kc.code());
            let below_code = axis_config.below.map(|kc| kc.code());
            
            axis_code.map(|code| (code, config::AxisCodeConfig {
                above: above_code,
                below: below_code,
            }))
        })
        .collect();

    println!("Mapped {} axes from config", axis_map.len());

    let addr = format!("{}:{}", config.server.listen_addr, config.server.listen_port);
    let addr: std::net::SocketAddr = addr.parse()?;
    
    let server = SocketServer::bind(addr, button_map, axis_map).await?;
    
    println!("Bouton server listening on {}", addr);
    server.run().await?;
    
    Ok(())
}
