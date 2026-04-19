use crate::keycode::KeyCode;
use bouton_core::control::GamepadControl;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StickId {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StickMapping {
    pub deadzone: u8,
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TriggerMapping {
    pub key: KeyCode,
    pub deadzone: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DPadMapping {
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mappings {
    pub listen_addr: String,
    pub listen_port: u16,
    pub buttons: HashMap<GamepadControl, KeyCode>,
    pub sticks: HashMap<StickId, StickMapping>,
    pub triggers: HashMap<GamepadControl, TriggerMapping>,
    pub dpad: Option<DPadMapping>,
    pub last_busid: Option<String>,
    pub last_device: Option<String>,
}

impl Default for Mappings {
    fn default() -> Self {
        let mut buttons = HashMap::new();
        buttons.insert(GamepadControl::Square, KeyCode::A);
        buttons.insert(GamepadControl::Cross, KeyCode::S);
        buttons.insert(GamepadControl::Circle, KeyCode::D);
        buttons.insert(GamepadControl::Triangle, KeyCode::W);
        buttons.insert(GamepadControl::L1, KeyCode::Q);
        buttons.insert(GamepadControl::R1, KeyCode::E);
        buttons.insert(GamepadControl::Select, KeyCode::Tab);
        buttons.insert(GamepadControl::Start, KeyCode::Enter);
        buttons.insert(GamepadControl::L3, KeyCode::LControl);
        buttons.insert(GamepadControl::R3, KeyCode::LAlt);

        let mut sticks = HashMap::new();
        sticks.insert(
            StickId::Left,
            StickMapping {
                deadzone: 20,
                up: KeyCode::W,
                down: KeyCode::S,
                left: KeyCode::A,
                right: KeyCode::D,
            },
        );
        sticks.insert(
            StickId::Right,
            StickMapping {
                deadzone: 20,
                up: KeyCode::Up,
                down: KeyCode::Down,
                left: KeyCode::Left,
                right: KeyCode::Right,
            },
        );

        let mut triggers = HashMap::new();
        triggers.insert(
            GamepadControl::L2,
            TriggerMapping {
                key: KeyCode::C,
                deadzone: 30,
            },
        );
        triggers.insert(
            GamepadControl::R2,
            TriggerMapping {
                key: KeyCode::V,
                deadzone: 30,
            },
        );

        Self {
            listen_addr: "0.0.0.0".to_string(),
            listen_port: 8000,
            buttons,
            sticks,
            triggers,
            dpad: Some(DPadMapping {
                up: KeyCode::Up,
                down: KeyCode::Down,
                left: KeyCode::Left,
                right: KeyCode::Right,
            }),
            last_busid: None,
            last_device: None,
        }
    }
}

pub fn state_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "cyy", "bouton")
        .map(|p| p.config_dir().join("state.json"))
}

pub async fn load() -> Mappings {
    if let Some(path) = state_path()
        && let Ok(bytes) = tokio::fs::read(&path).await
        && let Ok(m) = serde_json::from_slice::<Mappings>(&bytes)
    {
        return m;
    }
    Mappings::default()
}

pub async fn save(mappings: Mappings) {
    let Some(path) = state_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(&mappings) {
        let _ = tokio::fs::write(path, bytes).await;
    }
}
