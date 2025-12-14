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
    pub axes: HashMap<String, AxisConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisConfig {
    pub above: Option<KeyCode>,
    pub below: Option<KeyCode>,
}

#[derive(Debug, Clone)]
pub struct AxisCodeConfig {
    pub above: Option<u32>,
    pub below: Option<u32>,
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
listen_addr = "127.0.0.1"
listen_port = 8000

[keys.buttons]
# Map gamepad buttons to Windows Virtual Key codes.
# 
# Gamepad button names (standardized):
#   A, B, X, Y       - Face buttons
#   LB, RB           - Left/Right bumpers
#   LT, RT           - Left/Right triggers
#   Back, Start      - Menu buttons
#   LThumb, RThumb   - Stick press (L3, R3)
#   Guide, Home      - Guide/Home button
#
# Windows Virtual Key Code Reference:
# See: https://docs.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
#
# Common keys:
#   0x01-0x08  = Mouse buttons
#   0x09       = Tab
#   0x0D       = Enter
#   0x1B       = Escape
#   0x20       = Space
#   0x21-0x28  = Page Up/Down, End, Home, Arrows
#   0x30-0x39  = 0-9
#   0x41-0x5A  = A-Z
#   0x60-0x69  = Numpad 0-9
#   0x70-0x87  = F1-F24
#
# Examples:
#   13 = Enter
#   32 = Space
#   37 = Left Arrow
#   38 = Up Arrow
#   39 = Right Arrow
#   40 = Down Arrow
#   65 = A
#   87 = W

A = 13      # Enter
B = 32      # Space
X = 87      # W
Y = 83      # S
LB = 81     # Q
RB = 69     # E
LT = 65     # A
RT = 68     # D
Back = 27   # Escape
Start = 13  # Enter
LThumb = 32 # Space
RThumb = 32 # Space
"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_deserializes_from_toml() {
        let toml_str = r#"
[server]
listen_addr = "127.0.0.1"
listen_port = 8000

[keys.buttons]
A = 36
B = 38
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.listen_addr, "127.0.0.1");
        assert_eq!(config.server.listen_port, 8000);
        assert_eq!(config.keys.buttons.get("A"), Some(&36));
        assert_eq!(config.keys.buttons.get("B"), Some(&38));
    }
}
