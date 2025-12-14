use std::collections::HashMap;

pub fn key_to_code(key_name: &str) -> Option<u32> {
    KEYMAP.get(key_name).copied()
}

lazy_static::lazy_static! {
    static ref KEYMAP: HashMap<&'static str, u32> = {
        let mut m = HashMap::new();
        
        // Mouse buttons
        m.insert("LBUTTON", 0x01);
        m.insert("RBUTTON", 0x02);
        m.insert("MBUTTON", 0x04);
        m.insert("XBUTTON1", 0x05);
        m.insert("XBUTTON2", 0x06);
        
        // Standard keys
        m.insert("BACKSPACE", 0x08);
        m.insert("TAB", 0x09);
        m.insert("CLEAR", 0x0C);
        m.insert("ENTER", 0x0D);
        m.insert("RETURN", 0x0D);
        m.insert("SHIFT", 0x10);
        m.insert("CONTROL", 0x11);
        m.insert("CTRL", 0x11);
        m.insert("ALT", 0x12);
        m.insert("PAUSE", 0x13);
        m.insert("CAPSLOCK", 0x14);
        m.insert("ESCAPE", 0x1B);
        m.insert("ESC", 0x1B);
        m.insert("SPACE", 0x20);
        m.insert("PAGEUP", 0x21);
        m.insert("PAGEDOWN", 0x22);
        m.insert("END", 0x23);
        m.insert("HOME", 0x24);
        m.insert("LEFT", 0x25);
        m.insert("UP", 0x26);
        m.insert("RIGHT", 0x27);
        m.insert("DOWN", 0x28);
        m.insert("SELECT", 0x29);
        m.insert("PRINT", 0x2A);
        m.insert("EXECUTE", 0x2B);
        m.insert("PRINTSCREEN", 0x2C);
        m.insert("SCREENSHOT", 0x2C);
        m.insert("INSERT", 0x2D);
        m.insert("DELETE", 0x2E);
        m.insert("DEL", 0x2E);
        m.insert("HELP", 0x2F);
        
        // Number keys
        m.insert("0", 0x30);
        m.insert("1", 0x31);
        m.insert("2", 0x32);
        m.insert("3", 0x33);
        m.insert("4", 0x34);
        m.insert("5", 0x35);
        m.insert("6", 0x36);
        m.insert("7", 0x37);
        m.insert("8", 0x38);
        m.insert("9", 0x39);
        
        // Letter keys
        m.insert("A", 0x41);
        m.insert("B", 0x42);
        m.insert("C", 0x43);
        m.insert("D", 0x44);
        m.insert("E", 0x45);
        m.insert("F", 0x46);
        m.insert("G", 0x47);
        m.insert("H", 0x48);
        m.insert("I", 0x49);
        m.insert("J", 0x4A);
        m.insert("K", 0x4B);
        m.insert("L", 0x4C);
        m.insert("M", 0x4D);
        m.insert("N", 0x4E);
        m.insert("O", 0x4F);
        m.insert("P", 0x50);
        m.insert("Q", 0x51);
        m.insert("R", 0x52);
        m.insert("S", 0x53);
        m.insert("T", 0x54);
        m.insert("U", 0x55);
        m.insert("V", 0x56);
        m.insert("W", 0x57);
        m.insert("X", 0x58);
        m.insert("Y", 0x59);
        m.insert("Z", 0x5A);
        
        // Windows keys
        m.insert("LWIN", 0x5B);
        m.insert("RWIN", 0x5C);
        m.insert("APPS", 0x5D);
        m.insert("SLEEP", 0x5F);
        
        // Numpad
        m.insert("NUMPAD0", 0x60);
        m.insert("NUMPAD1", 0x61);
        m.insert("NUMPAD2", 0x62);
        m.insert("NUMPAD3", 0x63);
        m.insert("NUMPAD4", 0x64);
        m.insert("NUMPAD5", 0x65);
        m.insert("NUMPAD6", 0x66);
        m.insert("NUMPAD7", 0x67);
        m.insert("NUMPAD8", 0x68);
        m.insert("NUMPAD9", 0x69);
        m.insert("MULTIPLY", 0x6A);
        m.insert("ADD", 0x6B);
        m.insert("SEPARATOR", 0x6C);
        m.insert("SUBTRACT", 0x6D);
        m.insert("DECIMAL", 0x6E);
        m.insert("DIVIDE", 0x6F);
        
        // Function keys
        m.insert("F1", 0x70);
        m.insert("F2", 0x71);
        m.insert("F3", 0x72);
        m.insert("F4", 0x73);
        m.insert("F5", 0x74);
        m.insert("F6", 0x75);
        m.insert("F7", 0x76);
        m.insert("F8", 0x77);
        m.insert("F9", 0x78);
        m.insert("F10", 0x79);
        m.insert("F11", 0x7A);
        m.insert("F12", 0x7B);
        m.insert("F13", 0x7C);
        m.insert("F14", 0x7D);
        m.insert("F15", 0x7E);
        m.insert("F16", 0x7F);
        m.insert("F17", 0x80);
        m.insert("F18", 0x81);
        m.insert("F19", 0x82);
        m.insert("F20", 0x83);
        m.insert("F21", 0x84);
        m.insert("F22", 0x85);
        m.insert("F23", 0x86);
        m.insert("F24", 0x87);
        
        // Numlock and scroll lock
        m.insert("NUMLOCK", 0x90);
        m.insert("SCROLLLOCK", 0x91);
        
        // Shift/Ctrl/Alt variants
        m.insert("LSHIFT", 0xA0);
        m.insert("RSHIFT", 0xA1);
        m.insert("LCONTROL", 0xA2);
        m.insert("RCONTROL", 0xA3);
        m.insert("LALT", 0xA4);
        m.insert("RALT", 0xA5);
        
        // Browser keys
        m.insert("BROWSER_BACK", 0xA6);
        m.insert("BROWSER_FORWARD", 0xA7);
        m.insert("BROWSER_REFRESH", 0xA8);
        m.insert("BROWSER_STOP", 0xA9);
        m.insert("BROWSER_SEARCH", 0xAA);
        m.insert("BROWSER_FAVORITES", 0xAB);
        m.insert("BROWSER_HOME", 0xAC);
        
        // Volume and media
        m.insert("VOLUME_MUTE", 0xAD);
        m.insert("VOLUME_DOWN", 0xAE);
        m.insert("VOLUME_UP", 0xAF);
        m.insert("MEDIA_NEXT_TRACK", 0xB0);
        m.insert("MEDIA_PREV_TRACK", 0xB1);
        m.insert("MEDIA_STOP", 0xB2);
        m.insert("MEDIA_PLAY_PAUSE", 0xB3);
        m.insert("LAUNCH_MAIL", 0xB4);
        m.insert("LAUNCH_MEDIA_SELECT", 0xB5);
        m.insert("LAUNCH_APP1", 0xB6);
        m.insert("LAUNCH_APP2", 0xB7);
        
        // OEM keys
        m.insert("OEM_SEMICOLON", 0xBA);
        m.insert("OEM_EQUALS", 0xBB);
        m.insert("OEM_COMMA", 0xBC);
        m.insert("OEM_MINUS", 0xBD);
        m.insert("OEM_PERIOD", 0xBE);
        m.insert("OEM_SLASH", 0xBF);
        m.insert("OEM_BACKTICK", 0xC0);
        m.insert("OEM_LBRACKET", 0xDB);
        m.insert("OEM_BACKSLASH", 0xDC);
        m.insert("OEM_RBRACKET", 0xDD);
        m.insert("OEM_QUOTE", 0xDE);
        
        m
    };
}
