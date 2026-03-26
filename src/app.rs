use std::net::Ipv4Addr;

use crate::scanner::Host;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    ChoosingMode,
    Scanning,
    SelectingTargets,
    ConfirmingAttack,
    Attacking,
    Exiting,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KickMode {
    One,
    Some,
    All,
}

impl KickMode {
    pub const ALL_MODES: [KickMode; 3] = [KickMode::One, KickMode::Some, KickMode::All];

    pub fn label(&self) -> &'static str {
        match self {
            KickMode::One => "Kick ONE Device",
            KickMode::Some => "Kick SOME Devices",
            KickMode::All => "Kick ALL Devices",
        }
    }
}

#[derive(Debug, Clone)]
pub struct App {
    pub state: AppState,
    pub kick_mode: KickMode,
    pub mode_cursor: usize,
    pub hosts: Vec<Host>,
    pub selected_index: usize,
    pub selected_indices: Vec<usize>,
    pub targets: Vec<Host>,
    pub gateway_ip: Ipv4Addr,
    pub gateway_mac: Option<String>,
    pub interface_name: String,
    pub packets_per_min: u32,
    pub status_message: String,
    pub error_message: Option<String>,
}

impl App {
    pub fn new(interface_name: String, gateway_ip: Ipv4Addr) -> Self {
        Self {
            state: AppState::ChoosingMode,
            kick_mode: KickMode::One,
            mode_cursor: 0,
            hosts: Vec::new(),
            selected_index: 0,
            selected_indices: Vec::new(),
            targets: Vec::new(),
            gateway_ip,
            gateway_mac: None,
            interface_name,
            packets_per_min: 600,
            status_message: String::new(),
            error_message: None,
        }
    }

    pub fn next_mode(&mut self) {
        self.mode_cursor = (self.mode_cursor + 1) % KickMode::ALL_MODES.len();
        self.kick_mode = KickMode::ALL_MODES[self.mode_cursor];
    }

    pub fn previous_mode(&mut self) {
        if self.mode_cursor == 0 {
            self.mode_cursor = KickMode::ALL_MODES.len() - 1;
        } else {
            self.mode_cursor -= 1;
        }
        self.kick_mode = KickMode::ALL_MODES[self.mode_cursor];
    }

    pub fn next_host(&mut self) {
        if !self.hosts.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.hosts.len();
        }
    }

    pub fn previous_host(&mut self) {
        if !self.hosts.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.hosts.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn toggle_host_selection(&mut self) {
        if self.selected_indices.contains(&self.selected_index) {
            self.selected_indices.retain(|&i| i != self.selected_index);
        } else {
            self.selected_indices.push(self.selected_index);
        }
    }

    pub fn select_single_target(&mut self) {
        if let Some(host) = self.hosts.get(self.selected_index) {
            self.targets = vec![host.clone()];
        }
    }

    pub fn select_multiple_targets(&mut self) {
        self.targets = self
            .selected_indices
            .iter()
            .filter_map(|&i| self.hosts.get(i).cloned())
            .collect();
    }

    pub fn select_all_targets(&mut self) {
        self.targets = self
            .hosts
            .iter()
            .filter(|h| h.ip != self.gateway_ip)
            .cloned()
            .collect();
    }

    pub fn clear_selection(&mut self) {
        self.selected_indices.clear();
        self.targets.clear();
        self.selected_index = 0;
    }
}
