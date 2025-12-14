use crate::config::{DPadCodeConfig, JoystickCodeConfig, TriggerCodeConfig};
use crate::key_injector::KeyInjector;
use bouton_core::{ControlEvent, KeyAction, control::GamepadControl};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum UIEvent {
    ClientConnected(String),
    KeyPressed(String, u32),
    KeyReleased(String, u32),
    Unbound(String),
    Error(String),
}

pub struct SocketServer {
    socket: UdpSocket,
    button_map: Arc<HashMap<GamepadControl, u32>>,
    joystick_map: Arc<HashMap<GamepadControl, JoystickCodeConfig>>,
    trigger_map: Arc<HashMap<GamepadControl, TriggerCodeConfig>>,
    dpad_config: Arc<Option<DPadCodeConfig>>,
    ui_tx: mpsc::UnboundedSender<UIEvent>,
}

impl SocketServer {
    pub async fn bind(
        addr: SocketAddr,
        button_map: HashMap<GamepadControl, u32>,
        joystick_map: HashMap<GamepadControl, JoystickCodeConfig>,
        trigger_map: HashMap<GamepadControl, TriggerCodeConfig>,
        dpad_config: Option<DPadCodeConfig>,
        ui_tx: mpsc::UnboundedSender<UIEvent>,
    ) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self {
            socket,
            button_map: Arc::new(button_map),
            joystick_map: Arc::new(joystick_map),
            trigger_map: Arc::new(trigger_map),
            dpad_config: Arc::new(dpad_config),
            ui_tx,
        })
    }

    pub async fn run(self) -> std::io::Result<()> {
        let mut buf = [0u8; 4096];
        let button_map = Arc::new(self.button_map);
        let joystick_map = Arc::new(self.joystick_map);
        let trigger_map = Arc::new(self.trigger_map);
        let dpad_config = Arc::new(self.dpad_config);
        let ui_tx = self.ui_tx;

        // Track state across all datagrams
        let mut joystick_states: HashMap<GamepadControl, (u8, u8)> = HashMap::new();
        let mut joystick_pressed: HashMap<GamepadControl, (Option<u32>, Option<u32>)> =
            HashMap::new();
        let mut trigger_states: HashMap<GamepadControl, bool> = HashMap::new();
        let mut dpad_state: Option<(u8, u8)> = None;
        let mut dpad_pressed: Option<u32> = None;
        let mut connected_client: Option<std::net::SocketAddr> = None;

        loop {
            match self.socket.recv_from(&mut buf).await {
                Ok((n, addr)) => {
                    // Update when a new client connects or reconnects
                    if connected_client != Some(addr) {
                        connected_client = Some(addr);
                        let _ = ui_tx.send(UIEvent::ClientConnected(addr.to_string()));
                    }

                    if let Ok(event) = bincode::deserialize::<ControlEvent>(&buf[..n]) {
                        handle_event(
                            event,
                            &button_map,
                            &joystick_map,
                            &trigger_map,
                            &dpad_config,
                            &mut joystick_states,
                            &mut joystick_pressed,
                            &mut trigger_states,
                            &mut dpad_state,
                            &mut dpad_pressed,
                            ui_tx.clone(),
                        )
                        .await;
                    }
                }
                Err(e) => {
                    let _ = ui_tx.send(UIEvent::Error(format!("Socket recv error: {}", e)));
                }
            }
        }
    }
}

async fn handle_event(
    event: ControlEvent,
    button_map: &Arc<HashMap<GamepadControl, u32>>,
    joystick_map: &Arc<HashMap<GamepadControl, JoystickCodeConfig>>,
    trigger_map: &Arc<HashMap<GamepadControl, TriggerCodeConfig>>,
    dpad_config: &Arc<Option<DPadCodeConfig>>,
    joystick_states: &mut HashMap<GamepadControl, (u8, u8)>,
    joystick_pressed: &mut HashMap<GamepadControl, (Option<u32>, Option<u32>)>,
    trigger_states: &mut HashMap<GamepadControl, bool>,
    dpad_state: &mut Option<(u8, u8)>,
    dpad_pressed: &mut Option<u32>,
    ui_tx: mpsc::UnboundedSender<UIEvent>,
) {
    match event {
        ControlEvent::Button(button_event) => {
            if let Some(&key_code) = button_map.get(&button_event.control) {
                let key_name = code_to_name(key_code);

                match KeyInjector::inject(key_code, button_event.action) {
                    Err(e) => {
                        let _ = ui_tx.send(UIEvent::Error(format!("Failed to inject {}: {}", key_name, e)));
                    }
                    Ok(_) => {
                        let ui_event = match button_event.action {
                            KeyAction::Press => UIEvent::KeyPressed(key_name.clone(), key_code),
                            KeyAction::Release => UIEvent::KeyReleased(key_name.clone(), key_code),
                        };
                        let _ = ui_tx.send(ui_event);
                    }
                }
            } else {
                // Button is unbound
                let action_str = match button_event.action {
                    KeyAction::Press => "pressed",
                    KeyAction::Release => "released",
                };
                let _ = ui_tx.send(UIEvent::Unbound(format!("{} ({})", button_event.control, action_str)));
            }
        }
        ControlEvent::Axis(axis_event) => {
            let control = axis_event.control;

            match control {
                GamepadControl::LeftStickX | GamepadControl::LeftStickY => {
                    if let Some(joystick_config) = joystick_map.get(&GamepadControl::LeftStickX) {
                        handle_joystick_axis(
                            control,
                            axis_event.value as u8,
                            joystick_config,
                            joystick_states,
                            joystick_pressed,
                            ui_tx.clone(),
                        )
                        .await;
                    } else {
                        let _ = ui_tx.send(UIEvent::Unbound(format!("{}: {}", control, axis_event.value)));
                    }
                }
                GamepadControl::RightStickX | GamepadControl::RightStickY => {
                    if let Some(joystick_config) = joystick_map.get(&GamepadControl::RightStickX) {
                        handle_joystick_axis(
                            control,
                            axis_event.value as u8,
                            joystick_config,
                            joystick_states,
                            joystick_pressed,
                            ui_tx.clone(),
                        )
                        .await;
                    } else {
                        let _ = ui_tx.send(UIEvent::Unbound(format!("{}: {}", control, axis_event.value)));
                    }
                }
                GamepadControl::L2 | GamepadControl::R2 => {
                    if let Some(trigger_config) = trigger_map.get(&control) {
                        handle_trigger_axis(
                            control,
                            axis_event.value as u8,
                            trigger_config,
                            trigger_states,
                            ui_tx.clone(),
                        )
                        .await;
                    } else {
                        let _ = ui_tx.send(UIEvent::Unbound(format!("{}: {}", control, axis_event.value)));
                    }
                }
                GamepadControl::DPadX | GamepadControl::DPadY => {
                    if let Some(dpad) = dpad_config.as_ref() {
                        handle_dpad_axis(
                            control,
                            axis_event.value as u8,
                            dpad,
                            dpad_state,
                            dpad_pressed,
                            ui_tx.clone(),
                        )
                        .await;
                    } else {
                        let _ = ui_tx.send(UIEvent::Unbound(format!("{}: {}", control, axis_event.value)));
                    }
                }
                _ => {}
            }
        }
    }
}

async fn handle_joystick_axis(
    control: GamepadControl,
    value: u8,
    config: &JoystickCodeConfig,
    states: &mut HashMap<GamepadControl, (u8, u8)>,
    pressed: &mut HashMap<GamepadControl, (Option<u32>, Option<u32>)>,
    ui_tx: mpsc::UnboundedSender<UIEvent>,
) {
    // Determine the paired axis and use the X axis control as the key
    let (is_x_axis, stick_key) = match control {
        GamepadControl::LeftStickX => (true, GamepadControl::LeftStickX),
        GamepadControl::LeftStickY => (false, GamepadControl::LeftStickX),
        GamepadControl::RightStickX => (true, GamepadControl::RightStickX),
        GamepadControl::RightStickY => (false, GamepadControl::RightStickX),
        _ => return,
    };

    // Get current state, defaulting to center (127, 127)
    let (mut x, mut y) = states.get(&stick_key).copied().unwrap_or((127, 127));

    if is_x_axis {
        x = value;
    } else {
        y = value;
    }

    states.insert(stick_key, (x, y));

    // Check if we're in deadzone
    let center = 127i16;
    let x_diff = (x as i16 - center).abs();
    let y_diff = (y as i16 - center).abs();
    let x_in_deadzone = x_diff < config.deadzone as i16;
    let y_in_deadzone = y_diff < config.deadzone as i16;

    let (mut x_key_pressed, mut y_key_pressed) =
        pressed.get(&stick_key).copied().unwrap_or((None, None));

    // Handle X axis
    let new_x_key = if x_in_deadzone {
        None
    } else if x > 127 {
        Some(config.right)
    } else {
        Some(config.left)
    };

    if new_x_key != x_key_pressed {
        if let Some(old_key) = x_key_pressed {
            let key_name = code_to_name(old_key);
            match KeyInjector::inject(old_key, KeyAction::Release) {
                Err(e) => {
                    let _ = ui_tx.send(UIEvent::Error(format!("Failed to release {}: {}", key_name, e)));
                }
                Ok(_) => {
                    let _ = ui_tx.send(UIEvent::KeyReleased(key_name.clone(), old_key));
                }
            }
        }
        if let Some(new_key) = new_x_key {
            let key_name = code_to_name(new_key);
            match KeyInjector::inject(new_key, KeyAction::Press) {
                Err(e) => {
                    let _ = ui_tx.send(UIEvent::Error(format!("Failed to inject {}: {}", key_name, e)));
                }
                Ok(_) => {
                    let _ = ui_tx.send(UIEvent::KeyPressed(key_name.clone(), new_key));
                }
            }
        }
        x_key_pressed = new_x_key;
    }

    // Handle Y axis
    let new_y_key = if y_in_deadzone {
        None
    } else if y > 127 {
        Some(config.down)
    } else {
        Some(config.up)
    };

    if new_y_key != y_key_pressed {
        if let Some(old_key) = y_key_pressed {
            let key_name = code_to_name(old_key);
            match KeyInjector::inject(old_key, KeyAction::Release) {
                Err(e) => {
                    let _ = ui_tx.send(UIEvent::Error(format!("Failed to release {}: {}", key_name, e)));
                }
                Ok(_) => {
                    let _ = ui_tx.send(UIEvent::KeyReleased(key_name.clone(), old_key));
                }
            }
        }
        if let Some(new_key) = new_y_key {
            let key_name = code_to_name(new_key);
            match KeyInjector::inject(new_key, KeyAction::Press) {
                Err(e) => {
                    let _ = ui_tx.send(UIEvent::Error(format!("Failed to inject {}: {}", key_name, e)));
                }
                Ok(_) => {
                    let _ = ui_tx.send(UIEvent::KeyPressed(key_name.clone(), new_key));
                }
            }
        }
        y_key_pressed = new_y_key;
    }

    pressed.insert(stick_key, (x_key_pressed, y_key_pressed));
}

async fn handle_trigger_axis(
    control: GamepadControl,
    value: u8,
    config: &TriggerCodeConfig,
    states: &mut HashMap<GamepadControl, bool>,
    ui_tx: mpsc::UnboundedSender<UIEvent>,
) {
    let was_pressed = states.get(&control).copied().unwrap_or(false);
    let is_pressed = value > config.deadzone;

    // Only inject on state change
    if is_pressed != was_pressed {
        let action = if is_pressed {
            KeyAction::Press
        } else {
            KeyAction::Release
        };

        let key_name = code_to_name(config.key);
        match KeyInjector::inject(config.key, action) {
            Err(e) => {
                let _ = ui_tx.send(UIEvent::Error(format!("Failed to inject {}: {}", key_name, e)));
            }
            Ok(_) => {
                let ui_event = match action {
                    KeyAction::Press => UIEvent::KeyPressed(key_name.clone(), config.key),
                    KeyAction::Release => UIEvent::KeyReleased(key_name.clone(), config.key),
                };
                let _ = ui_tx.send(ui_event);
            }
        }

        states.insert(control, is_pressed);
    }
}

async fn handle_dpad_axis(
    control: GamepadControl,
    value: u8,
    config: &DPadCodeConfig,
    state: &mut Option<(u8, u8)>,
    pressed: &mut Option<u32>,
    ui_tx: mpsc::UnboundedSender<UIEvent>,
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
    let new_key = if x != 0 {
        // Horizontal input
        if x > 127 {
            Some(config.left)
        } else {
            Some(config.right)
        }
    } else if y != 0 {
        // Vertical input
        if y > 127 {
            Some(config.up)
        } else {
            Some(config.down)
        }
    } else {
        // Neutral, release any pressed key
        None
    };

    // Only inject if key state changed
     if new_key != *pressed {
         if let Some(old_key) = *pressed {
             let key_name = code_to_name(old_key);
             match KeyInjector::inject(old_key, KeyAction::Release) {
                 Err(e) => {
                     let _ = ui_tx.send(UIEvent::Error(format!("Failed to release {}: {}", key_name, e)));
                 }
                 Ok(_) => {
                     let _ = ui_tx.send(UIEvent::KeyReleased(key_name.clone(), old_key));
                 }
             }
         }
         if let Some(new_key_code) = new_key {
             let key_name = code_to_name(new_key_code);
             match KeyInjector::inject(new_key_code, KeyAction::Press) {
                 Err(e) => {
                     let _ = ui_tx.send(UIEvent::Error(format!("Failed to inject {}: {}", key_name, e)));
                 }
                 Ok(_) => {
                     let _ = ui_tx.send(UIEvent::KeyPressed(key_name.clone(), new_key_code));
                 }
             }
         }
         *pressed = new_key;
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
