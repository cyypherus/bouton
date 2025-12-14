WINDOWS:
winget install dorssel.usbipd-win # Install utility usbipd to move usb devices to wsl
usbipd list # List usb devices to find gamepad
(ADMIN) usbipd bind --busid 2-3 # Connect gamepad to wsl
usbipd attach --wsl --busid=2-3 # Connect connect connect

WSL:
ip route show | grep default # Outputs the port with which to reach windows
ls -la /dev/input/event* # Find the gamepad
sudo bouton-linux /dev/input/event0 # Start bouton with the identified gamepad
