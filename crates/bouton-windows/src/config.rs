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
    pub threshold: Option<u8>,
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
    pub threshold: u8,
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
        r#"
[server]
listen_addr = "0.0.0.0"
listen_port = 8000

[keys.buttons]
# Map gamepad buttons to Windows keys
# Use key names in SCREAMING_SNAKE_CASE format
# Examples: SPACE, ENTER, A, B, LEFT, RIGHT, UP, DOWN, F1-F24, etc.

Square = "A"
Cross = "S"
Circle = "D"
Triangle = "W"
L1 = "Q"
R1 = "E"
L2 = "C"
R2 = "V"
Select = "TAB"
Start = "ENTER"
L3 = "LCTRL"
R3 = "LALT"

[keys.joysticks]
# Joystick mappings with deadzone and 4-directional keys
# Rest position: 127 (center)
# Rests at 127, sends events on crossing thresholds

[keys.joysticks.LeftStick]
deadzone = 20
up = "W"
down = "S"
left = "A"
right = "D"

[keys.joysticks.RightStick]
deadzone = 20
up = "UP"
down = "DOWN"
left = "LEFT"
right = "RIGHT"

[keys.triggers]
# Trigger mappings (L2, R2)
# Rest position: 0
# Threshold: value above this triggers the key

[keys.triggers.L2]
key = "C"
threshold = 150

[keys.triggers.R2]
key = "V"
threshold = 150

[keys.dpad]
# D-Pad mappings
# Rest position: 0
# No deadzone, responds to any value change

[keys.dpad.DPad]
up = "UP"
down = "DOWN"
left = "LEFT"
right = "RIGHT"
"#
    }
}
