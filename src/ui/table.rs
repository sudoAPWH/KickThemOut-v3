use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

use crate::app::{AppState, KickMode};
use crate::scanner::Host;

pub struct HostTable;

impl HostTable {
    pub fn render(
        f: &mut Frame,
        hosts: &[Host],
        gateway_ip: std::net::Ipv4Addr,
        selected_index: usize,
        selected_indices: &[usize],
        state: AppState,
        kick_mode: KickMode,
        area: Rect,
    ) {
        let title = format!("Detected Hosts (Gateway: {})", gateway_ip);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        // Define columns widths
        let widths = if kick_mode == KickMode::Some && state == AppState::SelectingTargets {
            [
                ratatui::layout::Constraint::Length(3),  // checkbox
                ratatui::layout::Constraint::Length(5),  // ID
                ratatui::layout::Constraint::Length(18), // IP
                ratatui::layout::Constraint::Length(20), // MAC
                ratatui::layout::Constraint::Min(20),    // Vendor
            ]
        } else {
            [
                ratatui::layout::Constraint::Length(0),  // no checkbox
                ratatui::layout::Constraint::Length(5),  // ID
                ratatui::layout::Constraint::Length(18), // IP
                ratatui::layout::Constraint::Length(20), // MAC
                ratatui::layout::Constraint::Min(20),    // Vendor
            ]
        };

        let show_checkbox = kick_mode == KickMode::Some && state == AppState::SelectingTargets;

        let header = Row::new(vec![
            Cell::from(if show_checkbox { " " } else { "" }),
            Cell::from("ID").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("IP Address").style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Cell::from("MAC Address").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Cell::from("Vendor").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]);

        let is_selecting = state == AppState::SelectingTargets;

        let rows: Vec<Row> = hosts
            .iter()
            .enumerate()
            .map(|(i, host)| {
                let is_cursor = i == selected_index;
                let is_checked = selected_indices.contains(&i);
                let is_gateway = host.ip == gateway_ip;

                let ip_str = if is_gateway {
                    format!("{} (GW)", host.ip)
                } else {
                    host.ip.to_string()
                };

                let checkbox = if show_checkbox {
                    if is_checked { "[x]" } else { "[ ]" }
                } else {
                    ""
                };

                let style = if is_selecting && is_cursor {
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else if is_selecting && is_checked {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(checkbox).style(style),
                    Cell::from(format!("{}", i + 1)).style(style),
                    Cell::from(ip_str).style(style),
                    Cell::from(host.mac.clone()).style(style),
                    Cell::from(host.vendor.clone()).style(style),
                ])
            })
            .collect();

        let table = Table::new(rows, widths)
            .header(header)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        let mut table_state = TableState::default();
        if is_selecting {
            table_state.select(Some(selected_index));
        }

        f.render_stateful_widget(table, area, &mut table_state);
    }
}
