use bouton_core::{GamepadEvent, ControlEvent};
use evdev::Device;
use std::path::Path;

pub struct GamepadReader {
    device: Device,
}

impl GamepadReader {
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let device = Device::open(path)?;
        Ok(Self { device })
    }

    pub fn fetch_events(&mut self) -> std::io::Result<impl Iterator<Item = ControlEvent> + '_> {
        let events = self.device.fetch_events()?;
        Ok(events.filter_map(|e| {
            GamepadEvent::from_evdev(e).and_then(|event| event.to_control())
        }))
    }
}
