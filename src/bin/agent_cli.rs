//! Agent CLI with interactive TUI for monitoring and configuration

use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use parking_lot::Mutex;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame, Terminal,
};
use smotra_agent::Result;
use smotra_agent::{Agent, Config};
use std::collections::VecDeque;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;
use std::{io, sync::Arc};
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

// Tab indices
const TAB_STATUS: usize = 0;
const TAB_ENDPOINTS: usize = 1;
const TAB_CONFIG: usize = 2;
const TAB_LOGS: usize = 3;

#[derive(Parser)]
#[command(name = "agent-cli")]
#[command(about = "Smotra Agent CLI - Interactive monitoring interface", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the interactive TUI
    Tui,

    /// Show current status
    Status,

    /// Validate configuration
    ValidateConfig,

    /// Generate default configuration
    GenConfig {
        /// Output file path
        #[arg(short, long, default_value = "config.toml")]
        output: PathBuf,
    },
}

/// Log entry with level and message
#[derive(Clone)]
struct LogEntry {
    level: Level,
    message: String,
    timestamp: chrono::DateTime<chrono::Local>,
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {:5} {}",
            self.timestamp.format("%H:%M:%S"),
            self.level,
            self.message
        )
    }
}

/// Custom tracing layer that captures logs into a buffer
struct LogBuffer {
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    max_entries: usize,
}

impl LogBuffer {
    fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(max_entries))),
            max_entries,
        }
    }

    fn add_entry(&self, level: Level, message: String) {
        let mut entries = self.entries.lock();
        if entries.len() >= self.max_entries {
            entries.pop_front();
        }
        entries.push_back(LogEntry {
            level,
            message,
            timestamp: chrono::Local::now(),
        });
    }

    fn clone_handle(&self) -> Arc<Mutex<VecDeque<LogEntry>>> {
        Arc::clone(&self.entries)
    }
}

impl<S> tracing_subscriber::Layer<S> for LogBuffer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let level = *metadata.level();

        // Extract message from the event
        struct MessageVisitor(String);
        impl tracing::field::Visit for MessageVisitor {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
                if field.name() == "message" {
                    let msg = format!("{:?}", value);
                    // Remove surrounding quotes if present
                    self.0 = msg.trim_matches('"').to_string();
                }
            }
        }

        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);

        self.add_entry(level, visitor.0);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Tui) | None => {
            // For TUI mode, use in-memory log buffer
            let log_entries = init_tui_logging(&cli.log_level);
            run_tui(cli.config, log_entries).await?
        }
        Some(Commands::Status) => {
            // For non-TUI commands, use regular stdout logging
            init_stdout_logging(&cli.log_level);
            show_status(cli.config).await?
        }
        Some(Commands::ValidateConfig) => {
            init_stdout_logging(&cli.log_level);
            validate_config(cli.config).await?
        }
        Some(Commands::GenConfig { output }) => {
            init_stdout_logging(&cli.log_level);
            generate_config(output).await?
        }
    }

    Ok(())
}

/// Initialize stdout logging for non-TUI commands
fn init_stdout_logging(log_level: &str) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(log_level.parse().unwrap()),
        )
        .init();
}

fn init_tui_logging(log_level: &str) -> Arc<Mutex<VecDeque<LogEntry>>> {
    let log_buffer = LogBuffer::new(1000);
    let log_entries = log_buffer.clone_handle();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(log_level.parse().unwrap()),
        )
        .with(log_buffer)
        .init();

    log_entries
}

/// Run the interactive TUI
async fn run_tui(config_path: PathBuf, log_entries: Arc<Mutex<VecDeque<LogEntry>>>) -> Result<()> {
    // Load configuration
    let config = if config_path.exists() {
        Config::from_file(&config_path)?
    } else {
        tracing::error!("Config file not found at: {}", config_path.display());
        return Err(smotra_agent::Error::Config(format!(
            "Configuration file {} not found",
            config_path.display()
        )));
    };

    config.validate()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create agent
    let agent = Agent::new(config);

    // Run the UI
    let result = run_ui_loop(&mut terminal, agent, log_entries).await;

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

/// Main UI loop
async fn run_ui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    agent: Agent,
    log_entries: Arc<Mutex<VecDeque<LogEntry>>>,
) -> Result<()> {
    let mut selected_tab = 0;
    let mut config_scroll_offset = 0usize;
    let tabs = vec!["Status", "Endpoints", "Configuration", "Logs"];
    let agent = Arc::new(agent);

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
            render_header(f, chunks[0], &tabs, selected_tab);

            // Render content based on selected tab
            match selected_tab {
                TAB_STATUS => render_status(f, chunks[1], &status),
                TAB_ENDPOINTS => render_endpoints(f, chunks[1], &config),
                TAB_CONFIG => render_config(f, chunks[1], &config, config_scroll_offset),
                TAB_LOGS => render_logs(f, chunks[1], &logs),
                _ => {}
            }

            // Render footer
            render_footer(f, chunks[2]);
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
                            // let agent_clone = Agent::new(config.clone());
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

fn render_header(f: &mut Frame, area: Rect, tabs: &[&str], selected: usize) {
    let mut spans = Vec::new();

    for (i, &tab) in tabs.iter().enumerate() {
        let style = if i == selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        if i > 0 {
            spans.push(Span::raw(" | "));
        }
        spans.push(Span::raw(" "));
        spans.push(Span::styled(tab, style));
        spans.push(Span::raw(" "));
    }

    let header = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Smotra Agent CLI"),
    );

    f.render_widget(header, area);
}

fn render_status(f: &mut Frame, area: Rect, status: &smotra_agent::AgentStatus) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    // Status
    let status_text = if status.is_running {
        "Running"
    } else {
        "Stopped"
    };
    let status_color = if status.is_running {
        Color::Green
    } else {
        Color::Red
    };
    let status_widget = Paragraph::new(Line::from(Span::styled(
        status_text,
        Style::default()
            .fg(status_color)
            .add_modifier(Modifier::BOLD),
    )))
    .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(status_widget, chunks[0]);

    // Server connection
    let server_text = if status.server_connected {
        "Connected"
    } else {
        "Disconnected"
    };
    let server_color = if status.server_connected {
        Color::Green
    } else {
        Color::Yellow
    };
    let server_widget = Paragraph::new(Line::from(Span::styled(
        server_text,
        Style::default().fg(server_color),
    )))
    .block(Block::default().borders(Borders::ALL).title("Server"));
    f.render_widget(server_widget, chunks[1]);

    // Statistics
    let total = status.checks_performed;
    let success_rate = if total > 0 {
        (status.checks_successful as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let stats_text = format!(
        "Total: {} | Success: {} | Failed: {} | Rate: {:.1}%",
        total, status.checks_successful, status.checks_failed, success_rate
    );
    let stats_widget = Paragraph::new(stats_text)
        .block(Block::default().borders(Borders::ALL).title("Statistics"));
    f.render_widget(stats_widget, chunks[2]);

    // Success rate gauge
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Success Rate"))
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(success_rate / 100.0);
    f.render_widget(gauge, chunks[3]);
}

fn render_endpoints(f: &mut Frame, area: Rect, config: &Config) {
    let items: Vec<ListItem> = config
        .endpoints
        .iter()
        .map(|ep| {
            let port_str = ep.port.map(|p| format!(":{}", p)).unwrap_or_default();
            let tags_str = if ep.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", ep.tags.join(", "))
            };
            ListItem::new(format!("{}{}{}", ep.address, port_str, tags_str))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Endpoints ({})", config.endpoints.len())),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn render_config(f: &mut Frame, area: Rect, config: &Config, scroll_offset: usize) {
    // Serialize the full config structure to TOML format
    let config_str = match toml::to_string_pretty(config) {
        Ok(s) => s,
        Err(e) => format!("Error serializing config: {}", e),
    };

    // Convert the TOML string into lines for display
    let all_lines: Vec<String> = config_str.lines().map(|s| s.to_string()).collect();
    let total_lines = all_lines.len();

    // Calculate visible area (subtract 2 for borders)
    let visible_height = area.height.saturating_sub(2) as usize;

    // Clamp scroll offset to valid range
    let max_scroll = total_lines.saturating_sub(visible_height);
    let clamped_offset = scroll_offset.min(max_scroll);

    // Get the visible slice of lines
    let visible_lines: Vec<Line> = all_lines
        .iter()
        .skip(clamped_offset)
        .take(visible_height)
        .map(|line| Line::from(line.clone()))
        .collect();

    let title = if total_lines > visible_height {
        format!(
            "Configuration (lines {}-{}/{}, ↑↓/j/k to scroll)",
            clamped_offset + 1,
            (clamped_offset + visible_height).min(total_lines),
            total_lines
        )
    } else {
        "Configuration".to_string()
    };

    let paragraph =
        Paragraph::new(visible_lines).block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(paragraph, area);
}

fn render_logs(f: &mut Frame, area: Rect, logs: &[LogEntry]) {
    // Calculate visible height (subtract 2 for borders)
    let visible_height = area.height.saturating_sub(2) as usize;

    // Take only the last N logs that fit in the visible area
    let visible_logs = if logs.len() > visible_height {
        &logs[logs.len() - visible_height..]
    } else {
        logs
    };

    let items: Vec<ListItem> = visible_logs
        .iter()
        .map(|entry| {
            let color = match entry.level {
                Level::ERROR => Color::Red,
                Level::WARN => Color::Yellow,
                Level::INFO => Color::Green,
                Level::DEBUG => Color::Cyan,
                Level::TRACE => Color::Gray,
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp.format("%H:%M:%S")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:5} ", entry.level),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&entry.message, Style::default().fg(Color::White)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Logs ({} entries)", logs.len())),
    );

    f.render_widget(list, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let help_text = Line::from(vec![
        Span::raw(" ["),
        Span::styled("q/Esc", Style::default().fg(Color::Yellow)),
        Span::raw("] Quit | ["),
        Span::styled("←/→", Style::default().fg(Color::Yellow)),
        Span::raw("] Navigate | ["),
        Span::styled("↑↓/j/k", Style::default().fg(Color::Yellow)),
        Span::raw("] Scroll | ["),
        Span::styled("s", Style::default().fg(Color::Yellow)),
        Span::raw("] Start"),
    ]);

    let footer = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));

    f.render_widget(footer, area);
}

/// Show current status
async fn show_status(config_path: PathBuf) -> Result<()> {
    let config = Config::from_file(&config_path)?;
    let agent = Agent::new(config);
    let status = agent.status();

    println!("Agent Status:");
    println!("  Running: {}", status.is_running);
    println!("  Checks Performed: {}", status.checks_performed);
    println!("  Checks Successful: {}", status.checks_successful);
    println!("  Checks Failed: {}", status.checks_failed);
    println!("  Server Connected: {}", status.server_connected);
    println!("  Cached Results: {}", status.cached_results);

    Ok(())
}

/// Validate configuration
async fn validate_config(config_path: PathBuf) -> Result<()> {
    match Config::from_file(&config_path) {
        Ok(config) => match config.validate() {
            Ok(_) => println!("✓ Configuration is valid"),
            Err(e) => {
                eprintln!("✗ Configuration validation failed: {}", e);
                return Err(e);
            }
        },
        Err(e) => {
            eprintln!("✗ Failed to load configuration: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Generate default configuration
async fn generate_config(output: PathBuf) -> Result<()> {
    let config = Config::default();
    config.save_to_file(&output)?;
    println!("Generated default configuration at: {}", output.display());
    Ok(())
}
