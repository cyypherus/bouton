pub mod mapper;
pub mod control;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyAction {
    Press,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonEvent {
    pub button_code: u16,
    pub action: KeyAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxisEvent {
    pub axis_code: u16,
    pub value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputEvent {
    #[serde(rename = "button")]
    Button(ButtonEvent),
    #[serde(rename = "axis")]
    Axis(AxisEvent),
}

impl ButtonEvent {
    pub fn new(button_code: u16, action: KeyAction) -> Self {
        Self { button_code, action }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GamepadEvent {
    Button { code: u16, pressed: bool },
    Axis { code: u16, value: i32 },
}

#[cfg(feature = "evdev-support")]
impl GamepadEvent {
    pub fn from_evdev(event: evdev::InputEvent) -> Option<Self> {
        match event.event_type() {
            evdev::EventType::KEY => {
                Some(GamepadEvent::Button {
                    code: event.code(),
                    pressed: event.value() != 0,
                })
            }
            evdev::EventType::ABSOLUTE => {
                Some(GamepadEvent::Axis {
                    code: event.code(),
                    value: event.value(),
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_event_serializes_to_json() {
        let event = KeyEvent::new(36, KeyAction::Press);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("36"));
        assert!(json.contains("Press"));
    }

    #[test]
    fn key_event_deserializes_from_json() {
        let json = r#"{"key_code":36,"action":"Press"}"#;
        let event: KeyEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.key_code, 36);
        assert_eq!(event.action, KeyAction::Press);
    }

    #[test]
    fn key_action_press_and_release_are_distinct() {
        let press = KeyAction::Press;
        let release = KeyAction::Release;
        assert_ne!(press, release);
    }
}
