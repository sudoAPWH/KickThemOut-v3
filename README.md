# KickThemOut v3

**Easily remove unwanted devices from your local network.**

A modern python-based tool to disconnect devices from your local network using ARP spoofing.

## Features
- **Fast Scanning:** Pure Python ARP scan (no nmap required).
- **Modern UI:** Clean, interactive interface with vendor detection.
- **Safe:** Automatic network restoration on exit.
- **Simple:** Just one script.

## Quick Start

1.  **Download Repo:**
    ```bash
    git clone 'https://github.com/sudoAPWH/KickThemOut-v3.git' && cd KickThemOut-v3
    ```

2.  **Install Requirements:**
    ```bash
    pip3 install -r requirements.txt
    ```

3.  **Run (needs root/sudo):**
    ```bash
    sudo python3 kickthemout_v3.py

## Usage
Select a target from the list and choose a packet rate (default: 600/min). Press `Ctrl+C` to stop and restore connectivity.

## Troubleshooting

**Target stays online?**
Your computer might be forwarding packets instead of dropping them. You must disable IP forwarding for the attack to work.

- **macOS:**
  ```bash
  sudo sysctl -w net.inet.ip.forwarding=0
  ```

- **Linux:**
  ```bash
  sudo sysctl -w net.ipv4.ip_forward=0
  ```

## Disclaimer
For educational use and authorized network administration only. While inspired by the original [KickThemOut](https://github.com/k4m4/kickthemout) by Nikolaos Kamarinakis & David Schütz, this is a standalone project and not a fork, as the original project has been unmaintained for over six years, this version has been built from the ground up to provide a modern, maintained alternative.
