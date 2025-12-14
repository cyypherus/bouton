use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use crate::keycode::KeyCode;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub keys: KeyMappingConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub listen_addr: String,
    pub listen_port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyMappingConfig {
    pub buttons: HashMap<String, KeyCode>,
    #[serde(default)]
    pub joysticks: HashMap<String, JoystickConfig>,
    #[serde(default)]
    pub triggers: HashMap<String, TriggerConfig>,
    #[serde(default)]
    pub dpad: HashMap<String, DPadConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoystickConfig {
    pub deadzone: Option<u8>,
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    pub key: KeyCode,
    pub deadzone: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DPadConfig {
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
}

#[derive(Debug, Clone, Copy)]
pub struct JoystickCodeConfig {
    pub deadzone: u8,
    pub up: u32,
    pub down: u32,
    pub left: u32,
    pub right: u32,
}

#[derive(Debug, Clone)]
pub struct TriggerCodeConfig {
    pub key: u32,
    pub deadzone: u8,
}

#[derive(Debug, Clone)]
pub struct DPadCodeConfig {
    pub up: u32,
    pub down: u32,
    pub left: u32,
    pub right: u32,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn default_toml() -> &'static str {
        include_str!("../default.toml")
    }
}
