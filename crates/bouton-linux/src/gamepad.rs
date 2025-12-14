use bouton_core::{GamepadEvent, control::GamepadControl};
use evdev::Device;
use std::path::Path;

pub struct GamepadReader {
    device: Device,
    deadzone: i32,
}

impl GamepadReader {
    pub fn open_with_deadzone(path: impl AsRef<Path>, deadzone: i32) -> std::io::Result<Self> {
        let device = Device::open(path)?;
        Ok(Self { device, deadzone })
    }

    pub fn fetch_events(&mut self) -> std::io::Result<impl Iterator<Item = GamepadEvent> + '_> {
        let events = self.device.fetch_events()?;
        let deadzone = self.deadzone;
        Ok(events.filter_map(move |e| {
            GamepadEvent::from_evdev(e).and_then(|event| {
                match event {
                    GamepadEvent::Axis { code, value } => {
                        // Apply deadzone only to analog sticks (center around 127)
                        if let Some(control) = GamepadControl::from_code(code) {
                            if control.is_analog_stick() && deadzone > 0 {
                                // For analog sticks, check distance from center (127)
                                let distance_from_center = (value - 127).abs();
                                if distance_from_center < deadzone {
                                    return None;
                                }
                            }
                        }
                        
                        Some(GamepadEvent::Axis { code, value })
                    }
                    other => Some(other),
                }
            })
        }))
    }
}
