//! TUI main loop and event handling

use crate::logging::LogEntry;
use crate::tui::render;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use parking_lot::Mutex;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use smotra_agent::{Agent, Result};
use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use std::time::Duration;

// Tab indices
const TAB_STATUS: usize = 0;
const TAB_ENDPOINTS: usize = 1;
const TAB_CONFIG: usize = 2;
const TAB_LOGS: usize = 3;

/// Main UI loop
pub async fn run_ui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    agent: Arc<Agent>,
    log_entries: Arc<Mutex<VecDeque<LogEntry>>>,
) -> Result<()> {
    let mut selected_tab = 0;
    let mut config_scroll_offset = 0usize;
    let tabs = vec!["Status", "Endpoints", "Configuration", "Logs"];
    // let agent = Arc::new(agent);

    loop {
        // Update data
        let status = agent.status();
        let config = agent.config();
        let logs: Vec<LogEntry> = log_entries.lock().iter().cloned().collect();

        terminal.draw(|f| {
            let size = f.area();

            // Create main layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(0),    // Content
                    Constraint::Length(3), // Footer
                ])
                .split(size);

            // Render header with tabs
            render::render_header(f, chunks[0], &tabs, selected_tab);

            // Render content based on selected tab
            match selected_tab {
                TAB_STATUS => render::render_status(f, chunks[1], &status),
                TAB_ENDPOINTS => render::render_endpoints(f, chunks[1], &config),
                TAB_CONFIG => render::render_config(f, chunks[1], &config, config_scroll_offset),
                TAB_LOGS => render::render_logs(f, chunks[1], &logs),
                _ => {}
            }

            // Render footer
            render::render_footer(f, chunks[2]);
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Only handle KeyPress events, ignore KeyRelease and KeyRepeat
                if key.kind != event::KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        agent.stop()?;
                        break;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        agent.stop()?;
                        break;
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        selected_tab = selected_tab.saturating_sub(1);
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if selected_tab < tabs.len() - 1 {
                            selected_tab += 1;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if selected_tab == TAB_CONFIG {
                            // Configuration tab - scroll up
                            config_scroll_offset = config_scroll_offset.saturating_sub(1);
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if selected_tab == TAB_CONFIG {
                            // Configuration tab - scroll down
                            config_scroll_offset = config_scroll_offset.saturating_add(1);
                        }
                    }
                    KeyCode::PageUp => {
                        if selected_tab == TAB_CONFIG {
                            config_scroll_offset = config_scroll_offset.saturating_sub(10);
                        }
                    }
                    KeyCode::PageDown => {
                        if selected_tab == TAB_CONFIG {
                            config_scroll_offset = config_scroll_offset.saturating_add(10);
                        }
                    }
                    KeyCode::Home => {
                        if selected_tab == TAB_CONFIG {
                            config_scroll_offset = 0;
                        }
                    }
                    KeyCode::Char('s') => {
                        if !status.is_running {
                            // Start agent in background
                            let agent = Arc::clone(&agent);
                            tokio::spawn(async move {
                                let _ = agent.start().await;
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
