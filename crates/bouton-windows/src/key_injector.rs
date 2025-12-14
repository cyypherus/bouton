use bouton_core::KeyAction;

pub struct KeyInjector;

impl KeyInjector {
    pub fn inject(key_code: u32, action: KeyAction) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::Input::KeyboardAndMouse::{
                SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
            };

            let flags = match action {
                KeyAction::Press => 0u32,
                KeyAction::Release => KEYEVENTF_KEYUP.0,
            };

            let mut input = INPUT::default();
            input.r#type = INPUT_KEYBOARD;

            unsafe {
                input.Anonymous.ki = KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(
                        key_code as u16,
                    ),
                    wScan: 0,
                    dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(flags),
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
            println!(
                "Key event (non-Windows): code={}, action={:?}",
                key_code, action
            );
            Ok(())
        }
    }
}
