use thiserror::Error;

#[derive(Error, Debug)]
pub enum KickThemOutError {
    #[error("Root privileges required. Run with sudo.")]
    PermissionDenied,

    #[error("No suitable network interface found")]
    NoInterfaceFound,

    #[error("Failed to get interface info: {0}")]
    InterfaceError(String),

    #[error("ARP scan failed: {0}")]
    ArpScanError(String),

    #[error("Failed to send ARP packet: {0}")]
    ArpSendError(String),

    #[error("Gateway MAC address not found")]
    GatewayMacNotFound,

    #[error("No hosts found on network")]
    NoHostsFound,

    #[error("Invalid target selection: {0}")]
    InvalidTarget(String),

    #[error("Vendor lookup failed: {0}")]
    VendorLookupError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Network error: {0}")]
    NetworkError(String),
}

impl KickThemOutError {
    pub fn user_message(&self) -> String {
        match self {
            Self::PermissionDenied => {
                "This tool requires root privileges.\nRun with: sudo kickthemout".to_string()
            }
            Self::NoInterfaceFound => {
                "No suitable network interface found.\nPlease specify an interface with --interface <name>".to_string()
            }
            Self::GatewayMacNotFound => {
                "Could not detect gateway MAC address.\nEnsure your gateway is online and responding to ARP.".to_string()
            }
            _ => format!("Error: {}", self),
        }
    }
}

pub type Result<T> = std::result::Result<T, KickThemOutError>;