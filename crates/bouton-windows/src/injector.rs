use bouton_core::KeyAction;

pub fn inject(key_code: u32, action: KeyAction) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
            VIRTUAL_KEY,
        };

        let flags = match action {
            KeyAction::Press => 0u32,
            KeyAction::Release => KEYEVENTF_KEYUP.0,
        };

        let mut input = INPUT::default();
        input.r#type = INPUT_KEYBOARD;

        unsafe {
            input.Anonymous.ki = KEYBDINPUT {
                wVk: VIRTUAL_KEY(key_code as u16),
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(flags),
                time: 0,
                dwExtraInfo: 0,
            };
            let result = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            if result == 0 {
                return Err("SendInput failed".to_string());
            }
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (key_code, action);
        Ok(())
    }
}
