use bouton_core::{GamepadEvent, ControlEvent};
use evdev::Device;
use std::path::Path;
use tokio::sync::mpsc;

pub struct GamepadReader {
    rx: mpsc::UnboundedReceiver<ControlEvent>,
}

impl GamepadReader {
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let mut device = Device::open(path)?;
        let (tx, rx) = mpsc::unbounded_channel();
        
        tokio::spawn(async move {
            loop {
                if let Ok(events) = device.fetch_events() {
                    for event in events {
                        if let Some(gamepad_event) = GamepadEvent::from_evdev(event) {
                            if let Some(control_event) = gamepad_event.to_control() {
                                let _ = tx.send(control_event);
                            }
                        }
                    }
                }
                // Small delay to prevent busy spinning
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        });
        
        Ok(Self { rx })
    }

    pub fn try_recv(&mut self) -> Vec<ControlEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            events.push(event);
        }
        events
    }
}
