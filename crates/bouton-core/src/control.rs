#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadControl {
    // Buttons
    Square,
    Cross,
    Circle,
    Triangle,
    L1,
    R1,
    L3,
    R3,
    Select,
    Start,
    Touch,
    Aux1,
    Aux2,

    // Analog sticks
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,

    // Triggers
    L2,
    R2,

    // D-Pad
    DPadX,
    DPadY,
}

impl GamepadControl {
    pub fn code(&self) -> u16 {
        match self {
            // Buttons
            GamepadControl::Square => 0x130,
            GamepadControl::Cross => 0x131,
            GamepadControl::Circle => 0x132,
            GamepadControl::Triangle => 0x133,
            GamepadControl::L1 => 0x134,
            GamepadControl::R1 => 0x135,
            GamepadControl::L3 => 0x13A,
            GamepadControl::R3 => 0x13B,
            GamepadControl::Select => 0x138,
            GamepadControl::Start => 0x139,
            GamepadControl::Touch => 0x13D,
            GamepadControl::Aux1 => 0x13C,
            GamepadControl::Aux2 => 0x13E,

            // Analog sticks
            GamepadControl::LeftStickX => 0x00,
            GamepadControl::LeftStickY => 0x01,
            GamepadControl::RightStickX => 0x02,
            GamepadControl::RightStickY => 0x05,

            // Triggers
            GamepadControl::L2 => 0x03,
            GamepadControl::R2 => 0x04,

            // D-Pad
            GamepadControl::DPadX => 0x10,
            GamepadControl::DPadY => 0x11,
        }
    }

    pub fn from_code(code: u16) -> Option<Self> {
        match code {
            0x130 => Some(GamepadControl::Square),
            0x131 => Some(GamepadControl::Cross),
            0x132 => Some(GamepadControl::Circle),
            0x133 => Some(GamepadControl::Triangle),
            0x134 => Some(GamepadControl::L1),
            0x135 => Some(GamepadControl::R1),
            0x13A => Some(GamepadControl::L3),
            0x13B => Some(GamepadControl::R3),
            0x138 => Some(GamepadControl::Select),
            0x139 => Some(GamepadControl::Start),
            0x13D => Some(GamepadControl::Touch),
            0x13C => Some(GamepadControl::Aux1),
            0x13E => Some(GamepadControl::Aux2),

            0x00 => Some(GamepadControl::LeftStickX),
            0x01 => Some(GamepadControl::LeftStickY),
            0x02 => Some(GamepadControl::RightStickX),
            0x05 => Some(GamepadControl::RightStickY),

            0x03 => Some(GamepadControl::L2),
            0x04 => Some(GamepadControl::R2),

            0x10 => Some(GamepadControl::DPadX),
            0x11 => Some(GamepadControl::DPadY),

            _ => None,
        }
    }

    pub fn is_analog_stick(&self) -> bool {
        matches!(
            self,
            GamepadControl::LeftStickX
                | GamepadControl::LeftStickY
                | GamepadControl::RightStickX
                | GamepadControl::RightStickY
                | GamepadControl::L2
                | GamepadControl::R2
        )
    }
}
