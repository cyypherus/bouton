# Bouton

Bouton is an accessibility tool that converts gamepad input into keyboard events. It provides an alternative to tools like Joy2Key and AntimicroX, with a key difference: it completely hides HID events from Windows.

## Problem

Games and applications can detect gamepad input directly from the operating system. This means accessibility software that maps gamepad buttons to keyboard keys may be bypassed, as the application can still see and respond to the raw gamepad events. Bouton solves this by moving the gamepad to a Linux subsystem where it reads the raw input, then sends mapped key presses to Windows. Windows only sees keyboard events, not gamepad events, preventing any bypassing.

## How It Works

Bouton runs across two subsystems:

1. **Linux (WSL2)**: `bouton-linux` reads events from the connected gamepad and sends them to the Windows server
2. **Windows**: `bouton-windows` receives gamepad events and injects mapped keyboard key presses

The gamepad USB device is moved to the Linux subsystem using USBIPD-WIN. Windows applications only see the keyboard input, not the original gamepad events.

## Prerequisites

- Windows 10 or later
- WSL2 installed
- USBIPD-WIN installed (available via `winget install dorssel.usbipd-win`)
- A USB gamepad (only ps5 controller supported for now, PRs welcome)
- Rust installed (from https://rustup.rs/)

## Quick Start

### 1. Install

Clone the repository:
```
git clone https://github.com/sst/bouton
cd bouton
```

**On Windows (PowerShell):**
```powershell
cargo install --path crates/bouton-windows
cargo install --path crates/bouton-setup
```

**On WSL2/Linux:**
```bash
cargo install --path crates/bouton-linux
```

This installs all binaries to `~/.cargo/bin/` which should be in your PATH.

### 2. Run

You can either use the automated setup or run the commands manually.

**Option A: Automated Setup**

Run `bouton-setup` on Windows and it will walk you through the entire process.

**Option B: Manual Setup**

<details>
<summary>Click to expand manual setup steps</summary>

**Windows (PowerShell - Admin):**

```powershell
# Install USBIPD-WIN if not already installed
winget install dorssel.usbipd-win

# List USB devices to find your gamepad
usbipd list

# Bind the gamepad to WSL (replace 2-3 with your device's bus ID)
usbipd bind --busid 2-3

# Attach the gamepad to WSL
usbipd attach --wsl --busid=2-3

# Start the Windows server
bouton-windows
```

**WSL2/Linux:**

```bash
# Find the Windows IP address that WSL can reach
ip route show | grep default

# List input devices to find your gamepad
ls -la /dev/input/event*

# Start the Linux client (replace event0 with your gamepad's event number and IP with Windows IP)
sudo bouton-linux /dev/input/event0 192.168.X.X:8000
```

</details>

## Configuration

Create a `bouton.toml` file with your button to key mappings. See [default.toml](crates/bouton-windows/default.toml) for the full default configuration.

Example configuration:

```toml
[server]
listen_addr = "0.0.0.0"
listen_port = 8000

[keys.buttons]
Square = "A"
Cross = "S"
Circle = "D"
Triangle = "W"
L1 = "Q"
R1 = "E"

[keys.joysticks.LeftStick]
deadzone = 20
up = "W"
down = "S"
left = "A"
right = "D"

[keys.dpad.DPad]
up = "UP"
down = "DOWN"
left = "LEFT"
right = "RIGHT"
```

When `bouton-windows` runs without a config file, it creates a default `bouton.toml` with all available mappings and comments explaining each section.
