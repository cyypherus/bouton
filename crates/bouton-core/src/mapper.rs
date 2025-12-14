use crate::{ButtonEvent, AxisEvent, GamepadEvent, KeyAction, InputEvent};

pub struct GamepadMapper;

impl GamepadMapper {
    pub fn new() -> Self {
        Self
    }

    pub fn map_event(&mut self, event: &GamepadEvent) -> Option<InputEvent> {
        match event {
            GamepadEvent::Button { code, pressed } => {
                Some(InputEvent::Button(ButtonEvent::new(
                    *code,
                    if *pressed {
                        KeyAction::Press
                    } else {
                        KeyAction::Release
                    },
                )))
            }
            GamepadEvent::Axis { code, value } => {
                // Pass axis events through with the raw value (0-255)
                // Windows will handle threshold-based key mapping
                Some(InputEvent::Axis(AxisEvent {
                    axis_code: *code,
                    value: *value as u8,
                }))
            }
        }
    }
}

impl Default for GamepadMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_press_maps_to_button_event() {
        let mut mapper = GamepadMapper::new();
        let event = GamepadEvent::Button {
            code: 0x130,
            pressed: true,
        };
        let result = mapper.map_event(&event);
        assert_eq!(
            result,
            Some(InputEvent::Button(ButtonEvent::new(0x130, KeyAction::Press)))
        );
    }

    #[test]
    fn button_release_maps_to_button_release() {
        let mut mapper = GamepadMapper::new();
        let event = GamepadEvent::Button {
            code: 0x130,
            pressed: false,
        };
        let result = mapper.map_event(&event);
        assert_eq!(
            result,
            Some(InputEvent::Button(ButtonEvent::new(0x130, KeyAction::Release)))
        );
    }

    #[test]
    fn axis_events_passed_through() {
        let mut mapper = GamepadMapper::new();
        let event = GamepadEvent::Axis {
            code: 0x00,
            value: 200,
        };
        let result = mapper.map_event(&event);
        assert_eq!(
            result,
            Some(InputEvent::Axis(AxisEvent {
                axis_code: 0x00,
                value: 200,
            }))
        );
    }
}
