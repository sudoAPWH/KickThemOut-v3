use std::net::Ipv4Addr;
use std::time::Duration;

use pnet::datalink::{self, Channel::Ethernet, MacAddr};
use pnet::packet::arp::ArpPacket;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::Packet;

use crate::error::{KickThemOutError, Result};
use crate::scanner::{Host, NetworkInterface};

pub struct ArpScanner {
    interface: NetworkInterface,
}

impl ArpScanner {
    pub fn new(interface: NetworkInterface) -> Self {
        Self { interface }
    }

    /// Scan the network for hosts
    pub fn scan(&mut self) -> Result<Vec<Host>> {
        let interfaces = datalink::interfaces();
        let pnet_interface = interfaces
            .into_iter()
            .find(|iface| iface.name == self.interface.name)
            .ok_or(KickThemOutError::NoInterfaceFound)?;

        // Create datalink channel with a read timeout so rx.next() doesn't block forever
        let config = datalink::Config {
            read_timeout: Some(Duration::from_millis(200)),
            ..Default::default()
        };
        let (mut tx, mut rx) = match datalink::channel(&pnet_interface, config) {
            Ok(Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => {
                return Err(KickThemOutError::ArpScanError(
                    "Unsupported channel type".to_string(),
                ))
            }
            Err(e) => {
                return Err(KickThemOutError::ArpScanError(format!(
                    "Failed to create channel: {}",
                    e
                )))
            }
        };

        // First, send a dedicated ARP request to the gateway to make sure we get its MAC
        let gw_packet = self.build_arp_request(self.interface.gateway_ip);
        tx.send_to(&gw_packet, None);

        let subnet = self.interface.get_subnet_range();
        let hosts = self.scan_subnet(&mut tx, &mut rx, &subnet)?;

        // Identify gateway MAC from scan results
        for host in &hosts {
            if host.ip == self.interface.gateway_ip {
                if let Some(mac) = parse_mac_str(&host.mac) {
                    self.interface.gateway_mac = Some(mac);
                }
            }
        }

        // If gateway wasn't found in scan, try a targeted ARP request
        if self.interface.gateway_mac.is_none() {
            for _ in 0..3 {
                let gw_packet = self.build_arp_request(self.interface.gateway_ip);
                tx.send_to(&gw_packet, None);
            }

            let start = std::time::Instant::now();
            let gw_timeout = Duration::from_secs(2);
            while start.elapsed() < gw_timeout {
                match rx.next() {
                    Ok(packet) => {
                        if let Some(eth_packet) = EthernetPacket::new(packet) {
                            if eth_packet.get_ethertype() == EtherTypes::Arp {
                                if let Some(arp_packet) = ArpPacket::new(eth_packet.payload()) {
                                    if arp_packet.get_operation() == pnet::packet::arp::ArpOperations::Reply {
                                        let src_ip = Ipv4Addr::from(arp_packet.get_sender_proto_addr());
                                        if src_ip == self.interface.gateway_ip {
                                            let src_mac = arp_packet.get_sender_hw_addr();
                                            self.interface.gateway_mac = Some(src_mac);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => continue,
                }
            }
        }

        Ok(hosts)
    }

    fn scan_subnet(
        &self,
        tx: &mut Box<dyn datalink::DataLinkSender>,
        rx: &mut Box<dyn datalink::DataLinkReceiver>,
        subnet: &str,
    ) -> Result<Vec<Host>> {
        let mut hosts = Vec::new();

        // Parse subnet (e.g., "192.168.1.0/24")
        let parts: Vec<&str> = subnet.split('/').collect();
        if parts.len() != 2 {
            return Err(KickThemOutError::ArpScanError(
                "Invalid subnet format".to_string(),
            ));
        }

        let base_ip: Ipv4Addr = parts[0]
            .parse()
            .map_err(|e| KickThemOutError::ArpScanError(format!("Invalid IP: {}", e)))?;

        let prefix: u8 = parts[1]
            .parse()
            .map_err(|e| KickThemOutError::ArpScanError(format!("Invalid prefix: {}", e)))?;

        if prefix != 24 {
            return Err(KickThemOutError::ArpScanError(
                "Only /24 subnets are supported".to_string(),
            ));
        }

        // Generate list of IPs to scan
        let base_octets = base_ip.octets();

        // Send ARP requests for all IPs in the subnet
        for i in 1..255u8 {
            let target_ip = Ipv4Addr::new(base_octets[0], base_octets[1], base_octets[2], i);

            // Skip our own IP
            if target_ip == self.interface.ip {
                continue;
            }

            let packet = self.build_arp_request(target_ip);
            tx.send_to(&packet, None);
        }

        // Collect responses with timeout (no threading needed - just poll rx directly)
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(3);
        let my_ip = self.interface.ip;

        while start_time.elapsed() < timeout {
            match rx.next() {
                Ok(packet) => {
                    if let Some(eth_packet) = EthernetPacket::new(packet) {
                        if eth_packet.get_ethertype() == EtherTypes::Arp {
                            if let Some(arp_packet) = ArpPacket::new(eth_packet.payload()) {
                                if arp_packet.get_operation() == pnet::packet::arp::ArpOperations::Reply {
                                    let src_ip = Ipv4Addr::from(arp_packet.get_sender_proto_addr());
                                    let src_mac = arp_packet.get_sender_hw_addr();

                                    // Skip our own IP
                                    if src_ip == my_ip {
                                        continue;
                                    }

                                    hosts.push(Host {
                                        ip: src_ip,
                                        mac: src_mac.to_string(),
                                        vendor: "Unknown".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
                Err(_) => continue, // read timeout — check if overall timeout expired
            }
        }

        // Deduplicate hosts by MAC
        hosts.sort_by(|a, b| a.mac.cmp(&b.mac));
        hosts.dedup_by(|a, b| a.mac == b.mac);

        Ok(hosts)
    }

    fn build_arp_request(&self, target_ip: Ipv4Addr) -> Vec<u8> {
        let mut packet = Vec::with_capacity(42);

        // Ethernet header (14 bytes)
        // Destination MAC: broadcast
        packet.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        // Source MAC: our MAC
        packet.extend_from_slice(&self.interface.mac.octets());
        // EtherType: ARP (0x0806)
        packet.extend_from_slice(&[0x08, 0x06]);

        // ARP header (28 bytes)
        // Hardware type: Ethernet (1)
        packet.extend_from_slice(&[0x00, 0x01]);
        // Protocol type: IPv4 (0x0800)
        packet.extend_from_slice(&[0x08, 0x00]);
        // Hardware address length: 6
        packet.push(6);
        // Protocol address length: 4
        packet.push(4);
        // Operation: Request (1)
        packet.extend_from_slice(&[0x00, 0x01]);
        // Sender hardware address (our MAC)
        packet.extend_from_slice(&self.interface.mac.octets());
        // Sender protocol address (our IP)
        packet.extend_from_slice(&self.interface.ip.octets());
        // Target hardware address (unknown, use zeros)
        packet.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        // Target protocol address
        packet.extend_from_slice(&target_ip.octets());

        packet
    }

    /// Get the gateway MAC address (populated after scan)
    pub fn gateway_mac(&self) -> Option<String> {
        self.interface.gateway_mac.map(|m| m.to_string())
    }
}

fn parse_mac_str(mac: &str) -> Option<MacAddr> {
    let mac_str = mac.replace(':', "");
    let bytes = hex::decode(&mac_str).ok()?;
    if bytes.len() == 6 {
        Some(MacAddr::new(bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]))
    } else {
        None
    }
}