//! TUI rendering functions

use crate::logging::LogEntry;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};
use smotra_agent::Config;
use tracing::Level;

pub fn render_header(f: &mut Frame, area: Rect, tabs: &[&str], selected: usize) {
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

pub fn render_status(f: &mut Frame, area: Rect, status: &smotra_agent::AgentStatus, config: &Config) {
    use ratatui::layout::{Constraint, Direction, Layout};

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
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

    // Agent name
    let agent_name_widget = Paragraph::new(Line::from(Span::styled(
        &config.agent_name,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )))
    .block(Block::default().borders(Borders::ALL).title("Agent Name"));
    f.render_widget(agent_name_widget, chunks[1]);

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
    f.render_widget(server_widget, chunks[2]);

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
    f.render_widget(stats_widget, chunks[3]);

    // Success rate gauge
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Success Rate"))
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(success_rate / 100.0);
    f.render_widget(gauge, chunks[4]);
}

pub fn render_endpoints(f: &mut Frame, area: Rect, config: &Config) {
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

pub fn render_config(f: &mut Frame, area: Rect, config: &Config, scroll_offset: usize) {
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

pub fn render_logs(f: &mut Frame, area: Rect, logs: &[LogEntry]) {
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

pub fn render_footer(f: &mut Frame, area: Rect) {
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
