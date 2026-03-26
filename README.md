# KickThemOut v3

**Easily remove unwanted devices from your local network.**

A network administration tool written in Rust that disconnects devices from your local network using ARP spoofing. Features a modern interactive TUI with arrow-key navigation, multi-device selection, and automatic network restoration.

## Features
- **Fast ARP Scanning:** Discovers all devices on your /24 subnet with vendor detection.
- **Interactive TUI:** Arrow keys and spacebar to navigate and select targets -- no typing IPs or numbers.
- **Three Attack Modes:** Kick one device, select multiple, or kick all at once.
- **Safe:** Automatic ARP table restoration when you stop the attack.
- **Cross-Platform:** Supports macOS and Linux.

## Requirements
- **Rust toolchain** (1.70+) -- install from [rustup.rs](https://rustup.rs)
- **Root/sudo privileges** (required for raw packet access)

## Quick Start

1.  **Clone the repo:**
    ```bash
    git clone 'https://github.com/sudoAPWH/KickThemOut-v3.git' && cd KickThemOut-v3
    ```

2.  **Build and run:**
    ```bash
    sudo cargo run --release
    ```

    Or build first, then run the binary directly:
    ```bash
    cargo build --release
    sudo ./target/release/kickthemout
    ```

## Usage

1. **Choose a mode** -- use arrow keys to select, then press Enter:
   - **Kick ONE** -- select a single device to disconnect
   - **Kick SOME** -- select multiple devices
   - **Kick ALL** -- disconnect every device on the network

2. **Select targets** -- after the network scan completes:
   - **Arrow keys** to navigate the device list
   - **Space** to select (ONE mode) or toggle selection (SOME mode)
   - **Enter** to confirm your selection

3. **Confirm and attack** -- press `Y` to start, use arrow keys to adjust packets/min.

4. **Stop** -- press `Esc`, `Q`, or `Ctrl+C` to stop the attack and restore the network.

`Ctrl+C` exits cleanly from any screen.

## Troubleshooting

**Target stays online?**
Your computer might be forwarding packets instead of dropping them. Disable IP forwarding:

- **macOS:**
  ```bash
  sudo sysctl -w net.inet.ip.forwarding=0
  ```

- **Linux:**
  ```bash
  sudo sysctl -w net.ipv4.ip_forward=0
  ```

**No hosts found?**
Make sure you're connected to a network and running as root. The scanner only supports /24 subnets.

## Disclaimer
For educational use and authorized network administration only. While inspired by the original [KickThemOut](https://github.com/k4m4/kickthemout) by Nikolaos Kamarinakis & David Schutz, this is a standalone project built from the ground up.
