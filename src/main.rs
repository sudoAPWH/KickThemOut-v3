use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use kickthemout::{
    app::{App, AppState, KickMode},
    error::{KickThemOutError, Result},
    scanner::{ArpScanner, NetworkInterface, VendorResolver},
    spoofer::ArpSpoofer,
    ui::{HostTable, Menu},
};

fn main() -> Result<()> {
    // Check root privileges
    #[cfg(unix)]
    {
        use nix::unistd::Uid;
        if !Uid::effective().is_root() {
            eprintln!("ERROR: This tool requires root privileges.");
            eprintln!("Run with: sudo kickthemout");
            std::process::exit(1);
        }
    }

    // Setup terminal
    enable_raw_mode().map_err(|e| KickThemOutError::IoError(e))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| KickThemOutError::IoError(e))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| KickThemOutError::IoError(e))?;

    let running = Arc::new(AtomicBool::new(true));

    let result = run_app(&mut terminal, running.clone());

    // Restore terminal
    disable_raw_mode().map_err(|e| KickThemOutError::IoError(e))?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|e| KickThemOutError::IoError(e))?;
    terminal.show_cursor().map_err(|e| KickThemOutError::IoError(e))?;

    if let Err(e) = result {
        eprintln!("{}", e.user_message());
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    running: Arc<AtomicBool>,
) -> Result<()> {
    // Detect network interface early so we have gateway info for App
    let interface = NetworkInterface::detect()?;
    let gateway_ip = interface.gateway_ip;
    let interface_name = interface.name.clone();

    let mut app = App::new(interface_name, gateway_ip);
    let mut spoofer: Option<ArpSpoofer> = None;

    // Main loop
    while running.load(Ordering::SeqCst) {
        // Draw UI
        terminal
            .draw(|f| {
                let size = f.area();
                let show_table = !app.hosts.is_empty();

                if show_table {
                    let chunks = ratatui::layout::Layout::default()
                        .direction(ratatui::layout::Direction::Vertical)
                        .constraints([
                            ratatui::layout::Constraint::Min(10),
                            ratatui::layout::Constraint::Length(8),
                        ])
                        .split(size);

                    HostTable::render(
                        f,
                        &app.hosts,
                        app.gateway_ip,
                        app.selected_index,
                        &app.selected_indices,
                        app.state.clone(),
                        app.kick_mode,
                        chunks[0],
                    );

                    Menu::render(f, &app, chunks[1]);
                } else {
                    // No hosts yet — menu gets the full screen
                    Menu::render(f, &app, size);
                }
            })
            .map_err(|e| KickThemOutError::IoError(e))?;

        // If we're in Scanning state, do the scan (blocks briefly)
        if app.state == AppState::Scanning {
            let interface = NetworkInterface::detect()?;
            let mut scanner = ArpScanner::new(interface);
            let mut hosts = scanner.scan()?;

            if let Some(gw_mac) = scanner.gateway_mac() {
                app.gateway_mac = Some(gw_mac);
            }

            // Resolve vendors
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                KickThemOutError::NetworkError(format!("Failed to create tokio runtime: {}", e))
            })?;
            rt.block_on(async {
                let resolver = VendorResolver::new();
                resolver.resolve_batch(&mut hosts).await;
            });

            app.hosts = hosts;

            // After scan, go to appropriate state based on mode
            match app.kick_mode {
                KickMode::All => {
                    app.select_all_targets();
                    app.state = AppState::ConfirmingAttack;
                }
                _ => {
                    app.state = AppState::SelectingTargets;
                }
            }
            continue;
        }

        // Handle events
        if event::poll(Duration::from_millis(100)).map_err(|e| KickThemOutError::IoError(e))? {
            if let Event::Key(key) = event::read().map_err(|e| KickThemOutError::IoError(e))? {
                if key.kind == KeyEventKind::Press {
                    // Ctrl+C exits from anywhere
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        // Stop spoofer if running
                        if let Some(mut s) = spoofer.take() {
                            s.stop();
                        }
                        running.store(false, Ordering::SeqCst);
                        continue;
                    }

                    match app.state {
                        AppState::ChoosingMode => match key.code {
                            KeyCode::Up => app.previous_mode(),
                            KeyCode::Down => app.next_mode(),
                            KeyCode::Enter => {
                                // Transition to scanning
                                app.state = AppState::Scanning;
                            }
                            KeyCode::Esc => {
                                running.store(false, Ordering::SeqCst);
                            }
                            _ => {}
                        },
                        AppState::SelectingTargets => match key.code {
                            KeyCode::Up => app.previous_host(),
                            KeyCode::Down => app.next_host(),
                            KeyCode::Char(' ') => {
                                match app.kick_mode {
                                    KickMode::One => {
                                        // In ONE mode, space selects and confirms immediately
                                        app.select_single_target();
                                        if !app.targets.is_empty() {
                                            app.state = AppState::ConfirmingAttack;
                                        }
                                    }
                                    KickMode::Some => {
                                        app.toggle_host_selection();
                                    }
                                    KickMode::All => {} // shouldn't be here
                                }
                            }
                            KeyCode::Enter => {
                                match app.kick_mode {
                                    KickMode::One => {
                                        app.select_single_target();
                                        if !app.targets.is_empty() {
                                            app.state = AppState::ConfirmingAttack;
                                        }
                                    }
                                    KickMode::Some => {
                                        app.select_multiple_targets();
                                        if !app.targets.is_empty() {
                                            app.state = AppState::ConfirmingAttack;
                                        }
                                    }
                                    KickMode::All => {}
                                }
                            }
                            KeyCode::Esc => {
                                app.clear_selection();
                                app.hosts.clear();
                                app.state = AppState::ChoosingMode;
                            }
                            _ => {}
                        },
                        AppState::ConfirmingAttack => match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                if app.gateway_mac.is_none() {
                                    app.error_message =
                                        Some("Gateway MAC not found. Cannot start attack.".to_string());
                                    app.state = AppState::ChoosingMode;
                                } else {
                                    let mut interface = NetworkInterface::detect()?;
                                    // Set gateway MAC from scan results
                                    if let Some(ref gw_mac_str) = app.gateway_mac {
                                        let mac_str = gw_mac_str.replace(':', "");
                                        if let Ok(bytes) = hex::decode(&mac_str) {
                                            if bytes.len() == 6 {
                                                interface.gateway_mac = Some(pnet::datalink::MacAddr::new(
                                                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
                                                ));
                                            }
                                        }
                                    }
                                    let mut spoofer_instance =
                                        ArpSpoofer::new(interface, app.targets.clone());
                                    spoofer_instance.set_packets_per_min(app.packets_per_min);
                                    spoofer_instance.start()?;
                                    spoofer = Some(spoofer_instance);
                                    app.state = AppState::Attacking;
                                }
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                app.clear_selection();
                                app.hosts.clear();
                                app.state = AppState::ChoosingMode;
                            }
                            KeyCode::Up => {
                                if app.packets_per_min < 6000 {
                                    app.packets_per_min += 100;
                                }
                            }
                            KeyCode::Down => {
                                if app.packets_per_min > 100 {
                                    app.packets_per_min -= 100;
                                }
                            }
                            _ => {}
                        },
                        AppState::Attacking => match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                                if let Some(mut s) = spoofer.take() {
                                    s.stop();
                                }
                                app.clear_selection();
                                app.hosts.clear();
                                app.state = AppState::ChoosingMode;
                            }
                            _ => {}
                        },
                        AppState::Scanning | AppState::Exiting => {}
                    }
                }
            }
        }
    }

    // Cleanup
    if let Some(mut s) = spoofer {
        s.stop();
    }

    Ok(())
}
