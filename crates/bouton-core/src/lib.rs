pub mod control;

use serde::{Deserialize, Serialize};
use control::GamepadControl;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyAction {
    Press,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlButton {
    pub control: GamepadControl,
    pub action: KeyAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlAxis {
    pub control: GamepadControl,
    pub value: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlEvent {
    #[serde(rename = "button")]
    Button(ControlButton),
    #[serde(rename = "axis")]
    Axis(ControlAxis),
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

impl GamepadEvent {
    pub fn to_control(self) -> Option<ControlEvent> {
        match self {
            GamepadEvent::Button { code, pressed } => {
                let control = GamepadControl::from_code(code)?;
                let action = if pressed {
                    KeyAction::Press
                } else {
                    KeyAction::Release
                };
                Some(ControlEvent::Button(ControlButton { control, action }))
            }
            GamepadEvent::Axis { code, value } => {
                let control = GamepadControl::from_code(code)?;
                Some(ControlEvent::Axis(ControlAxis { control, value }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_button_event_serializes() {
        let event = ControlButton {
            control: GamepadControl::Square,
            action: KeyAction::Press,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Square"));
        assert!(json.contains("Press"));
    }

    #[test]
    fn key_action_press_and_release_are_distinct() {
        let press = KeyAction::Press;
        let release = KeyAction::Release;
        assert_ne!(press, release);
    }
}
