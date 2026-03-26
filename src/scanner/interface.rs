use std::net::Ipv4Addr;

use pnet::datalink::{interfaces, MacAddr};
use pnet::ipnetwork::IpNetwork;

use crate::error::{KickThemOutError, Result};
use crate::platform;

#[derive(Debug, Clone)]
pub struct Host {
    pub ip: Ipv4Addr,
    pub mac: String,
    pub vendor: String,
}

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: Ipv4Addr,
    pub mac: MacAddr,
    pub gateway_ip: Ipv4Addr,
    pub gateway_mac: Option<MacAddr>,
}

impl NetworkInterface {
    /// Detect the default network interface
    pub fn detect() -> Result<Self> {
        // Get interface name from platform-specific detection
        let interface_name = platform::get_default_interface_name()?;

        // Get all interfaces
        let interfaces = interfaces();

        // Find our interface
        let pnet_interface = interfaces
            .into_iter()
            .find(|iface| iface.name == interface_name)
            .ok_or_else(|| {
                KickThemOutError::InterfaceError(format!(
                    "Interface '{}' not found",
                    interface_name
                ))
            })?;

        Self::from_pnet_interface(pnet_interface)
    }

    /// Create from a specific interface name
    pub fn from_name(name: &str) -> Result<Self> {
        let interfaces = interfaces();
        let pnet_interface = interfaces
            .into_iter()
            .find(|iface| iface.name == name)
            .ok_or_else(|| {
                KickThemOutError::InterfaceError(format!("Interface '{}' not found", name))
            })?;

        Self::from_pnet_interface(pnet_interface)
    }

    fn from_pnet_interface(iface: pnet::datalink::NetworkInterface) -> Result<Self> {
        let name = iface.name.clone();

        // Get MAC address
        let mac = iface.mac.ok_or_else(|| {
            KickThemOutError::InterfaceError(format!("No MAC address for interface '{}'", name))
        })?;

        // Get IPv4 address
        let ip = iface
            .ips
            .iter()
            .find_map(|ip| match ip {
                IpNetwork::V4(ipv4) => Some(ipv4.ip()),
                _ => None,
            })
            .ok_or_else(|| {
                KickThemOutError::InterfaceError(format!(
                    "No IPv4 address for interface '{}'",
                    name
                ))
            })?;

        // Get gateway IP
        let gateway_ip = platform::get_default_gateway()?;

        Ok(Self {
            name,
            ip,
            mac,
            gateway_ip,
            gateway_mac: None,
        })
    }

    /// Get the subnet range for scanning (/24)
    pub fn get_subnet_range(&self) -> String {
        let octets = self.ip.octets();
        format!("{}.{}.{}.0/24", octets[0], octets[1], octets[2])
    }
}