use ratatui::{
    layout::Rect,
    prelude::Stylize,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppState, KickMode};

pub struct Menu;

impl Menu {
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let title = Span::styled(
            "KickThemOut v3.0.0",
            Style::default().fg(Color::Blue).bold(),
        );

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));

        let lines = match app.state {
            AppState::ChoosingMode => {
                let mut lines = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Select attack mode:",
                        Style::default().bold(),
                    )),
                ];

                for (i, mode) in KickMode::ALL_MODES.iter().enumerate() {
                    let is_selected = i == app.mode_cursor;
                    let prefix = if is_selected { "▸ " } else { "  " };
                    let label = format!("{}{}", prefix, mode.label());
                    let style = if is_selected {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(Span::styled(label, style)));
                }

                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "↑↓ Navigate  Enter Select  Ctrl+C Quit",
                    Style::default().fg(Color::DarkGray),
                )));
                lines
            }
            AppState::Scanning => {
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Scanning network...",
                        Style::default().fg(Color::Yellow).bold(),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Please wait, discovering hosts on your network.",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            }
            AppState::SelectingTargets => {
                let mode_hint = match app.kick_mode {
                    KickMode::One => "Select ONE device to kick",
                    KickMode::Some => "Select devices to kick (Space to toggle)",
                    KickMode::All => "", // shouldn't stay here
                };
                let mut lines = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        mode_hint,
                        Style::default().fg(Color::Yellow).bold(),
                    )),
                ];

                let nav_hint = match app.kick_mode {
                    KickMode::One => "↑↓ Navigate  Enter Select  Esc Back  Ctrl+C Quit",
                    KickMode::Some => "↑↓ Navigate  Space Toggle  Enter Confirm  Esc Back  Ctrl+C Quit",
                    KickMode::All => "",
                };
                lines.push(Line::from(""));
                if !app.selected_indices.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("  {} device(s) selected", app.selected_indices.len()),
                        Style::default().fg(Color::Green),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    nav_hint,
                    Style::default().fg(Color::DarkGray),
                )));
                lines
            }
            AppState::ConfirmingAttack => {
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("Targeting {} device(s)", app.targets.len()),
                        Style::default().fg(Color::Red).bold(),
                    )),
                    Line::from(format!("  Packets/min: {} (↑↓ to adjust)", app.packets_per_min)),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Y Confirm  N Cancel  Ctrl+C Quit",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            }
            AppState::Attacking => {
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Attack Running...",
                        Style::default().fg(Color::Green).bold(),
                    )),
                    Line::from(format!("  Targets: {}", app.targets.len())),
                    Line::from(format!("  Packets/min: {}", app.packets_per_min)),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Esc/Q Stop attack  Ctrl+C Quit",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            }
            AppState::Exiting => {
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Goodbye!",
                        Style::default().fg(Color::Blue).bold(),
                    )),
                ]
            }
        };

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }
}
