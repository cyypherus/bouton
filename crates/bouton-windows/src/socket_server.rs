use crate::key_injector::KeyInjector;
use crate::config::AxisCodeConfig;
use bouton_core::InputEvent;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

pub struct SocketServer {
    listener: TcpListener,
    button_map: Arc<HashMap<u16, u32>>,
    axis_map: Arc<HashMap<u16, AxisCodeConfig>>,
}

impl SocketServer {
    pub async fn bind(addr: SocketAddr, button_map: HashMap<u16, u32>, axis_map: HashMap<u16, AxisCodeConfig>) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self { 
            listener, 
            button_map: Arc::new(button_map),
            axis_map: Arc::new(axis_map),
        })
    }

    pub async fn run(self) -> std::io::Result<()> {
        loop {
            let (socket, addr) = self.listener.accept().await?;
            println!("Client connected: {}", addr);
            let button_map = Arc::clone(&self.button_map);
            let axis_map = Arc::clone(&self.axis_map);
            tokio::spawn(handle_connection(socket, button_map, axis_map));
        }
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    button_map: Arc<HashMap<u16, u32>>,
    axis_map: Arc<HashMap<u16, AxisCodeConfig>>,
) -> std::io::Result<()> {
    use tokio::io::AsyncBufReadExt;
    let reader = tokio::io::BufReader::new(&mut socket);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        match serde_json::from_str::<InputEvent>(&line) {
            Ok(event) => {
                match event {
                    InputEvent::Button(button_event) => {
                        println!("Button event received: code=0x{:X}, action={:?}", button_event.button_code, button_event.action);
                        
                        if let Some(&key_code) = button_map.get(&button_event.button_code) {
                            println!("  → Button mapped to key code {}", key_code);
                            
                            if let Err(e) = KeyInjector::inject(key_code, button_event.action) {
                                eprintln!("  ✗ Failed to inject key: {}", e);
                            } else {
                                println!("  ✓ Key injected");
                            }
                        } else {
                            println!("  → Button not mapped in config");
                        }
                    }
                    InputEvent::Axis(axis_event) => {
                        println!("Axis event received: code=0x{:X}, value={}", axis_event.axis_code, axis_event.value);
                        
                        if let Some(axis_config) = axis_map.get(&axis_event.axis_code) {
                            // Determine action based on which key is configured
                            let (key_code, action) = if let Some(above_key) = axis_config.above {
                                if let Some(below_key) = axis_config.below {
                                    // Both configured: threshold is midpoint (127)
                                    let is_above = axis_event.value > 127;
                                    if is_above {
                                        (above_key, bouton_core::KeyAction::Press)
                                    } else {
                                        (below_key, bouton_core::KeyAction::Release)
                                    }
                                } else {
                                    // Only above: always press when triggered
                                    (above_key, bouton_core::KeyAction::Press)
                                }
                            } else if let Some(below_key) = axis_config.below {
                                // Only below: always release when triggered
                                (below_key, bouton_core::KeyAction::Release)
                            } else {
                                // Neither configured
                                println!("  → Axis has no keys configured");
                                continue;
                            };
                            
                            println!("  → Axis value {} mapped to key code {}", 
                                axis_event.value, 
                                key_code);
                            
                            if let Err(e) = KeyInjector::inject(key_code, action) {
                                eprintln!("  ✗ Failed to inject key: {}", e);
                            } else {
                                println!("  ✓ Key injected");
                            }
                        } else {
                            println!("  → Axis not mapped in config");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to decode event: {}", e);
            }
        }
    }

    Ok(())
}
