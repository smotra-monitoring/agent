//! Logging infrastructure for TUI and CLI modes

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Log entry with level and message
#[derive(Clone)]
pub struct LogEntry {
    pub level: Level,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
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
pub struct LogBuffer {
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    max_entries: usize,
}

impl LogBuffer {
    pub fn new(max_entries: usize) -> Self {
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

    pub fn clone_handle(&self) -> Arc<Mutex<VecDeque<LogEntry>>> {
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

/// Initialize stdout logging for non-TUI commands
pub fn init_stdout_logging(log_level: &str) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(log_level.parse().unwrap()),
        )
        .init();
}

/// Initialize TUI logging with in-memory buffer
pub fn init_tui_logging(log_level: &str) -> Arc<Mutex<VecDeque<LogEntry>>> {
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
