use std::net::Ipv4Addr;
use std::process::Command;

use crate::error::{KickThemOutError, Result};

/// Check if the current process is running as root
pub fn check_root() -> Result<()> {
    #[cfg(unix)]
    {
        use nix::unistd::Uid;
        if !Uid::effective().is_root() {
            return Err(KickThemOutError::PermissionDenied);
        }
    }
    #[cfg(not(unix))]
    {
        return Err(KickThemOutError::PermissionDenied);
    }
    Ok(())
}

/// Get the default gateway IP address by finding the route to 8.8.8.8
pub fn get_default_gateway() -> Result<Ipv4Addr> {
    #[cfg(target_os = "macos")]
    {
        get_gateway_macos()
    }
    #[cfg(target_os = "linux")]
    {
        get_gateway_linux()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(KickThemOutError::NetworkError(
            "Unsupported operating system".to_string(),
        ))
    }
}

#[cfg(target_os = "macos")]
fn get_gateway_macos() -> Result<Ipv4Addr> {
    // Use route -n get 8.8.8.8 to find gateway
    let output = Command::new("route")
        .args(["-n", "get", "8.8.8.8"])
        .output()
        .map_err(|e| KickThemOutError::NetworkError(format!("Failed to execute route: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse output for "gateway: X.X.X.X"
    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("gateway:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1]
                    .parse()
                    .map_err(|e| KickThemOutError::NetworkError(format!("Invalid gateway IP: {}", e)));
            }
        }
    }

    Err(KickThemOutError::NetworkError(
        "Could not find gateway in route output".to_string(),
    ))
}

#[cfg(target_os = "linux")]
fn get_gateway_linux() -> Result<Ipv4Addr> {
    // Parse /proc/net/route to find default gateway
    let content = std::fs::read_to_string("/proc/net/route")
        .map_err(|e| KickThemOutError::NetworkError(format!("Failed to read /proc/net/route: {}", e)))?;

    // Format: Iface Destination Gateway Flags RefCnt Use Metric Mask MTU Window IRTT
    // Gateway is in hex, little-endian
    for line in content.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 3 {
            continue;
        }

        // Destination 00000000 means default route
        if fields[1] == "00000000" {
            let gateway_hex = fields[2];
            // Convert hex to IP (little-endian)
            let gateway = hex_to_ipv4(gateway_hex)?;
            return Ok(gateway);
        }
    }

    Err(KickThemOutError::NetworkError(
        "Could not find default gateway".to_string(),
    ))
}

#[cfg(target_os = "linux")]
fn hex_to_ipv4(hex: &str) -> Result<Ipv4Addr> {
    if hex.len() != 8 {
        return Err(KickThemOutError::NetworkError(format!(
            "Invalid hex IP length: {}",
            hex
        )));
    }

    // Parse hex string (little-endian)
    let bytes = u32::from_str_radix(hex, 16)
        .map_err(|e| KickThemOutError::NetworkError(format!("Invalid hex IP: {}", e)))?;

    // Convert from little-endian
    Ok(Ipv4Addr::from(bytes.to_le_bytes()))
}

/// Get the network interface used to reach the internet (8.8.8.8)
pub fn get_default_interface_name() -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        get_interface_macos()
    }
    #[cfg(target_os = "linux")]
    {
        get_interface_linux()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(KickThemOutError::NetworkError(
            "Unsupported operating system".to_string(),
        ))
    }
}

#[cfg(target_os = "macos")]
fn get_interface_macos() -> Result<String> {
    let output = Command::new("route")
        .args(["-n", "get", "8.8.8.8"])
        .output()
        .map_err(|e| KickThemOutError::NetworkError(format!("Failed to execute route: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("interface:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return Ok(parts[1].to_string());
            }
        }
    }

    Err(KickThemOutError::NoInterfaceFound)
}

#[cfg(target_os = "linux")]
fn get_interface_linux() -> Result<String> {
    let content = std::fs::read_to_string("/proc/net/route")
        .map_err(|e| KickThemOutError::NetworkError(format!("Failed to read /proc/net/route: {}", e)))?;

    for line in content.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 1 {
            continue;
        }

        // Destination 00000000 means default route
        if fields[1] == "00000000" {
            return Ok(fields[0].to_string());
        }
    }

    Err(KickThemOutError::NoInterfaceFound)
}