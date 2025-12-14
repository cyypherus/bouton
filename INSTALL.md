# Installation

## Build and Install

**On Windows (PowerShell):**
```powershell
cargo install --path crates/bouton-windows
cargo install --path crates/bouton-setup
```

**On WSL/Linux:**
```bash
cargo install --path crates/bouton-linux
```

This installs all three binaries to `~/.cargo/bin/` which should be in your PATH.

## Usage

1. Run `bouton-setup` from Windows
2. It will find `bouton-windows.exe` and `bouton-linux` in PATH automatically
3. Follow the prompts to configure USB device, IP, deadzone, config file
4. Setup will attach the USB device, launch the Windows server, and start the Linux client
