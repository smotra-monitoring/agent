//! TUI runner - handles terminal setup and teardown

use crate::logging::LogEntry;
use crate::tui;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use parking_lot::Mutex;
use ratatui::{backend::CrosstermBackend, Terminal};
use smotra::{Agent, Result};
use std::collections::VecDeque;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

/// Run the interactive TUI
pub async fn run_tui(
    config_path: PathBuf,
    log_entries: Arc<Mutex<VecDeque<LogEntry>>>,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create agent (loads and validates config internally)
    let agent = Arc::new(Agent::new(config_path)?);

    // Run the UI
    let result = tui::run_ui_loop(&mut terminal, agent, log_entries).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}
