# Bouton

Bouton is an accessibility tool that converts gamepad input into keyboard events — making it *impossible* for applications on Windows to intercept HID events from the gamepad. It shares use cases with software like Joy2Key / AntimicroX / HidHide but is geared towards more technical users.

<img width="500" height="400" alt="Screenshot 2025-12-14 152733" src="https://github.com/user-attachments/assets/541332c1-ce51-4fcd-840c-5f6111c2a1a4" />
<img width="500" height="400" alt="Screenshot 2025-12-14 152718" src="https://github.com/user-attachments/assets/5231fee1-2856-468d-8f65-b599c74b0b41" />

## Problem

Games and applications can detect gamepad input directly from the operating system. This means accessibility software that maps gamepad buttons to keyboard keys may be bypassed, as the application can still see and respond to the raw gamepad events. Bouton solves this by moving the gamepad to a Linux subsystem where it reads the raw input, then sends mapped key presses to Windows. Windows only sees keyboard events, not gamepad events, preventing any bypassing.

## How It Works

Bouton runs across two subsystems:

1. **Linux (WSL2)** — `bouton-linux` reads events from the connected gamepad and forwards them over UDP to the Windows server.
2. **Windows** — `bouton-windows` receives gamepad events and injects mapped keyboard key presses via `SendInput`.

The gamepad USB device is moved to the Linux subsystem using USBIPD-WIN. Windows applications only see keyboard input, never the original gamepad events.

## Prerequisites

- Windows 10 or later
- WSL2 with WSLg (for the Linux GUI)
- USBIPD-WIN (`winget install dorssel.usbipd-win`)
- A USB gamepad (PS5 controller confirmed; PRs welcome for others)
- Rust (https://rustup.rs/)

## Install

```
git clone https://github.com/cyypherus/bouton
cd bouton
```

**Windows (PowerShell):**

```powershell
cargo install --path crates/bouton-windows
```

**WSL2:**

```bash
cargo install --path crates/bouton-linux
```

## Run

1. Launch `bouton-windows` on Windows. Use the USB Setup panel to pick your gamepad's bus ID, bind it (elevated), and attach it to WSL.
2. In the Gamepad Client panel, enter the WSL device path (e.g. `/dev/input/event8`) and click **Launch WSL Client**. A new WSL terminal opens running `bouton-linux`. If you hit a permission-denied error on the device, either toggle **Run with sudo** in the panel, or add your user to the `input` group: `sudo usermod -aG input $USER` then run `wsl --shutdown` from Windows and reopen.
3. Configure mappings in the Windows GUI. Changes apply immediately and persist automatically. The Gamepad Client panel shows the last key event as feedback.

Mappings and connection state live under your platform's config directory (e.g. `%APPDATA%\cyy\bouton\config` on Windows).
