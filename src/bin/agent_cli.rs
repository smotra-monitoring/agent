//! Agent CLI with interactive TUI for monitoring and configuration

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame, Terminal,
};
use smotra_agent::{Agent, Config};
use std::io;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "agent-cli")]
#[command(about = "Smotra Agent CLI - Interactive monitoring interface", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
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

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Tui) => run_tui(cli.config).await?,
        Some(Commands::Status) => show_status(cli.config).await?,
        Some(Commands::ValidateConfig) => validate_config(cli.config).await?,
        Some(Commands::GenConfig { output }) => generate_config(output).await?,
        None => run_tui(cli.config).await?,
    }

    Ok(())
}

/// Run the interactive TUI
async fn run_tui(config_path: PathBuf) -> Result<()> {
    // Load configuration
    let config = if config_path.exists() {
        Config::from_file(&config_path)?
    } else {
        eprintln!("Config file not found, using default configuration");
        Config::default()
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create agent
    let agent = Agent::new(config);

    // Run the UI
    let result = run_ui(&mut terminal, agent).await;

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
async fn run_ui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, agent: Agent) -> Result<()> {
    let mut selected_tab = 0;
    let tabs = vec!["Status", "Endpoints", "Configuration", "Logs"];

    loop {
        let status = agent.status();
        let config = agent.config();

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
                0 => render_status(f, chunks[1], &status),
                1 => render_endpoints(f, chunks[1], &config),
                2 => render_config(f, chunks[1], &config),
                3 => render_logs(f, chunks[1]),
                _ => {}
            }

            // Render footer
            render_footer(f, chunks[2]);
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
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
                        if selected_tab > 0 {
                            selected_tab -= 1;
                        }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if selected_tab < tabs.len() - 1 {
                            selected_tab += 1;
                        }
                    }
                    KeyCode::Char('s') => {
                        if !status.is_running {
                            // Start agent in background
                            let agent_clone = Agent::new(config.clone());
                            tokio::spawn(async move {
                                let _ = agent_clone.start().await;
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

fn render_status(f: &mut Frame, area: Rect, status: &smotra_agent::types::AgentStatus) {
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

fn render_config(f: &mut Frame, area: Rect, config: &Config) {
    let config_text = vec![
        Line::from(format!("Agent ID: {}", config.agent_id)),
        Line::from(format!("Tags: {}", config.tags.join(", "))),
        Line::from(format!(
            "Check Interval: {}s",
            config.monitoring.interval_secs
        )),
        Line::from(format!(
            "Check Timeout: {}s",
            config.monitoring.timeout_secs
        )),
        Line::from(format!("Ping Count: {}", config.monitoring.ping_count)),
        Line::from(format!(
            "Max Concurrent: {}",
            config.monitoring.max_concurrent
        )),
        Line::from(format!(
            "Server URL: {}",
            config.server.url.as_deref().unwrap_or("not configured")
        )),
        Line::from(format!(
            "Report Interval: {}s",
            config.server.report_interval_secs
        )),
    ];

    let paragraph = Paragraph::new(config_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Configuration"),
    );

    f.render_widget(paragraph, area);
}

fn render_logs(f: &mut Frame, area: Rect) {
    let logs = vec![
        "Log display coming soon...",
        "This will show recent monitoring events",
    ];

    let items: Vec<ListItem> = logs.iter().map(|&l| ListItem::new(l)).collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Logs"));

    f.render_widget(list, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let help_text = Line::from(vec![
        Span::raw(" ["),
        Span::styled("q/Esc", Style::default().fg(Color::Yellow)),
        Span::raw("] Quit | ["),
        Span::styled("←/→", Style::default().fg(Color::Yellow)),
        Span::raw("] Navigate | ["),
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
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("✗ Failed to load configuration: {}", e);
            std::process::exit(1);
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
