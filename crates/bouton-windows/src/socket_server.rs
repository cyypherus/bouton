use crate::key_injector::KeyInjector;
use crate::config::{JoystickCodeConfig, TriggerCodeConfig, DPadCodeConfig};
use bouton_core::{InputEvent, control::GamepadControl, KeyAction};
use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

pub struct SocketServer {
    listener: TcpListener,
    button_map: Arc<HashMap<u16, u32>>,
    joystick_map: Arc<HashMap<u16, JoystickCodeConfig>>,
    trigger_map: Arc<HashMap<u16, TriggerCodeConfig>>,
    dpad_config: Arc<Option<DPadCodeConfig>>,
}

impl SocketServer {
    pub async fn bind(
        addr: SocketAddr,
        button_map: HashMap<u16, u32>,
        joystick_map: HashMap<u16, JoystickCodeConfig>,
        trigger_map: HashMap<u16, TriggerCodeConfig>,
        dpad_config: Option<DPadCodeConfig>,
    ) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self {
            listener,
            button_map: Arc::new(button_map),
            joystick_map: Arc::new(joystick_map),
            trigger_map: Arc::new(trigger_map),
            dpad_config: Arc::new(dpad_config),
        })
    }

    pub async fn run(self) -> std::io::Result<()> {
        loop {
            let (socket, addr) = self.listener.accept().await?;
            println!("Client connected: {}", addr);
            let button_map = Arc::clone(&self.button_map);
            let joystick_map = Arc::clone(&self.joystick_map);
            let trigger_map = Arc::clone(&self.trigger_map);
            let dpad_config = Arc::clone(&self.dpad_config);
            tokio::spawn(handle_connection(
                socket,
                button_map,
                joystick_map,
                trigger_map,
                dpad_config,
            ));
        }
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    button_map: Arc<HashMap<u16, u32>>,
    joystick_map: Arc<HashMap<u16, JoystickCodeConfig>>,
    trigger_map: Arc<HashMap<u16, TriggerCodeConfig>>,
    dpad_config: Arc<Option<DPadCodeConfig>>,
) -> std::io::Result<()> {
    use tokio::io::AsyncBufReadExt;
    let reader = tokio::io::BufReader::new(&mut socket);
    let mut lines = reader.lines();
    
    // Track axis states for threshold crossings
    let mut joystick_states: HashMap<GamepadControl, (u8, u8)> = HashMap::new(); // (x, y) values
    let mut trigger_states: HashMap<GamepadControl, bool> = HashMap::new(); // is_pressed
    let mut dpad_state: Option<(u8, u8)> = None; // (x, y) values

    while let Some(line) = lines.next_line().await? {
        match serde_json::from_str::<InputEvent>(&line) {
            Ok(event) => {
                match event {
                    InputEvent::Button(button_event) => {
                        if let Some(control) = GamepadControl::from_code(button_event.button_code) {
                            println!("Button event received: {:?}, action={:?}", control, button_event.action);
                        } else {
                            println!("Button event received: code=0x{:X}, action={:?}", button_event.button_code, button_event.action);
                        }
                        
                        if let Some(&key_code) = button_map.get(&button_event.button_code) {
                            let key_name = code_to_name(key_code);
                            println!("  → Button mapped to key: {}", key_name);
                            
                            if let Err(e) = KeyInjector::inject(key_code, button_event.action) {
                                eprintln!("  ✗ Failed to inject key: {}", e);
                            } else {
                                println!("  ✓ {} {}", key_name, if button_event.action == KeyAction::Press { "pressed" } else { "released" });
                            }
                        } else {
                            println!("  → Button not mapped in config");
                        }
                    }
                    InputEvent::Axis(axis_event) => {
                        let control = GamepadControl::from_code(axis_event.axis_code);
                        
                        if let Some(control) = control {
                            println!("Axis event received: {:?}, value={}", control, axis_event.value);
                            
                            match control {
                                GamepadControl::LeftStickX | GamepadControl::LeftStickY
                                | GamepadControl::RightStickX | GamepadControl::RightStickY => {
                                    if let Some(joystick_config) = joystick_map.get(&axis_event.axis_code) {
                                        handle_joystick_axis(control, axis_event.value, joystick_config, &mut joystick_states).await;
                                    }
                                }
                                GamepadControl::L2 | GamepadControl::R2 => {
                                    if let Some(trigger_config) = trigger_map.get(&axis_event.axis_code) {
                                        handle_trigger_axis(control, axis_event.value, trigger_config, &mut trigger_states).await;
                                    }
                                }
                                GamepadControl::DPadX | GamepadControl::DPadY => {
                                    if let Some(dpad) = dpad_config.as_ref() {
                                        handle_dpad_axis(control, axis_event.value, dpad, &mut dpad_state).await;
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            println!("Axis event received: code=0x{:X}, value={}", axis_event.axis_code, axis_event.value);
                            println!("  → Unknown axis code");
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

async fn handle_joystick_axis(
    control: GamepadControl,
    value: u8,
    config: &JoystickCodeConfig,
    states: &mut HashMap<GamepadControl, (u8, u8)>,
) {
    // Determine the paired axis
    let (paired_control, is_x_axis) = match control {
        GamepadControl::LeftStickX => (GamepadControl::LeftStickY, true),
        GamepadControl::LeftStickY => (GamepadControl::LeftStickX, false),
        GamepadControl::RightStickX => (GamepadControl::RightStickY, true),
        GamepadControl::RightStickY => (GamepadControl::RightStickX, false),
        _ => return,
    };
    
    // Get current state, defaulting to center (127, 127)
    let (mut x, mut y) = states.get(&control).copied().unwrap_or((127, 127));
    
    if is_x_axis {
        x = value;
    } else {
        y = value;
    }
    
    states.insert(control, (x, y));
    
    // Check if we're in deadzone
    let center = 127i16;
    let x_diff = (x as i16 - center).abs();
    let y_diff = (y as i16 - center).abs();
    let in_deadzone = x_diff < config.deadzone as i16 && y_diff < config.deadzone as i16;
    
    if in_deadzone {
        return; // Ignore inputs in deadzone
    }
    
    // Determine which direction to send
    let (key_code, direction) = if x_diff > y_diff {
        // More horizontal movement
        if x > 127 {
            (config.right, "right")
        } else {
            (config.left, "left")
        }
    } else {
        // More vertical movement
        if y > 127 {
            (config.down, "down")
        } else {
            (config.up, "up")
        }
    };
    
    let key_name = code_to_name(key_code);
    println!("  → Joystick {:?} moved {}, key: {}", control, direction, key_name);
    
    if let Err(e) = KeyInjector::inject(key_code, KeyAction::Press) {
        eprintln!("  ✗ Failed to inject key: {}", e);
    } else {
        println!("  ✓ {} pressed", key_name);
    }
}

async fn handle_trigger_axis(
    control: GamepadControl,
    value: u8,
    config: &TriggerCodeConfig,
    states: &mut HashMap<GamepadControl, bool>,
) {
    let was_pressed = states.get(&control).copied().unwrap_or(false);
    let is_pressed = value > config.threshold;
    
    // Only inject on state change
    if is_pressed != was_pressed {
        let action = if is_pressed {
            KeyAction::Press
        } else {
            KeyAction::Release
        };
        
        let key_name = code_to_name(config.key);
        println!("  → Trigger {:?} crossed threshold, key: {}", control, key_name);
        
        if let Err(e) = KeyInjector::inject(config.key, action) {
            eprintln!("  ✗ Failed to inject key: {}", e);
        } else {
            println!("  ✓ {} {}", key_name, if action == KeyAction::Press { "pressed" } else { "released" });
        }
        
        states.insert(control, is_pressed);
    }
}

async fn handle_dpad_axis(
    control: GamepadControl,
    value: u8,
    config: &DPadCodeConfig,
    state: &mut Option<(u8, u8)>,
) {
    // D-Pad tracking
    let (is_x_axis, _other_control) = match control {
        GamepadControl::DPadX => (true, GamepadControl::DPadY),
        GamepadControl::DPadY => (false, GamepadControl::DPadX),
        _ => return,
    };
    
    let (mut x, mut y) = state.unwrap_or((0, 0));
    
    if is_x_axis {
        x = value;
    } else {
        y = value;
    }
    
    *state = Some((x, y));
    
    // D-Pad: determine direction and send key
    let (key_code, direction) = if x != 0 {
        // Horizontal input
        if x > 127 {
            (config.right, "right")
        } else {
            (config.left, "left")
        }
    } else if y != 0 {
        // Vertical input
        if y > 127 {
            (config.down, "down")
        } else {
            (config.up, "up")
        }
    } else {
        // Neutral, no key
        return;
    };
    
    let key_name = code_to_name(key_code);
    println!("  → D-Pad moved {}, key: {}", direction, key_name);
    
    if let Err(e) = KeyInjector::inject(key_code, KeyAction::Press) {
        eprintln!("  ✗ Failed to inject key: {}", e);
    } else {
        println!("  ✓ {} pressed", key_name);
    }
}

fn code_to_name(code: u32) -> String {
    match code {
        0x01 => "LButton".to_string(),
        0x02 => "RButton".to_string(),
        0x04 => "MButton".to_string(),
        0x05 => "XButton1".to_string(),
        0x06 => "XButton2".to_string(),
        0x08 => "Backspace".to_string(),
        0x09 => "Tab".to_string(),
        0x0C => "Clear".to_string(),
        0x0D => "Enter".to_string(),
        0x10 => "Shift".to_string(),
        0x11 => "Ctrl".to_string(),
        0x12 => "Alt".to_string(),
        0x13 => "Pause".to_string(),
        0x14 => "CapsLock".to_string(),
        0x1B => "Esc".to_string(),
        0x20 => "Space".to_string(),
        0x21 => "PageUp".to_string(),
        0x22 => "PageDown".to_string(),
        0x23 => "End".to_string(),
        0x24 => "Home".to_string(),
        0x25 => "Left".to_string(),
        0x26 => "Up".to_string(),
        0x27 => "Right".to_string(),
        0x28 => "Down".to_string(),
        0x2C => "PrintScreen".to_string(),
        0x2D => "Insert".to_string(),
        0x2E => "Delete".to_string(),
        0x30..=0x39 => format!("{}", code - 0x30),
        0x41..=0x5A => format!("{}", (code as u8 as char)),
        0x60..=0x69 => format!("Numpad{}", code - 0x60),
        0x70..=0x87 => format!("F{}", code - 0x70 + 1),
        0x90 => "NumLock".to_string(),
        0x91 => "ScrollLock".to_string(),
        0xA0 => "LShift".to_string(),
        0xA1 => "RShift".to_string(),
        0xA2 => "LAlt".to_string(),
        0xA3 => "RAlt".to_string(),
        0xAD => "VolumeMute".to_string(),
        0xAE => "VolumeDown".to_string(),
        0xAF => "VolumeUp".to_string(),
        0xB0 => "MediaNextTrack".to_string(),
        0xB1 => "MediaPrevTrack".to_string(),
        0xB2 => "MediaStop".to_string(),
        0xB3 => "MediaPlayPause".to_string(),
        0xBA => ";".to_string(),
        0xBB => "=".to_string(),
        0xBC => ",".to_string(),
        0xBD => "-".to_string(),
        0xBE => ".".to_string(),
        0xBF => "/".to_string(),
        0xC0 => "`".to_string(),
        0xDB => "[".to_string(),
        0xDC => "\\".to_string(),
        0xDD => "]".to_string(),
        0xDE => "'".to_string(),
        _ => format!("Unknown(0x{:02X})", code),
    }
}
