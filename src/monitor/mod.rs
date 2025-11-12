//! Monitoring coordination and task management

mod coordinator;
mod ping;

pub use coordinator::run_monitoring;
pub use ping::PingChecker;

use crate::core::types::MonitoringResult;
use tokio::sync::mpsc;

/// Channel for sending monitoring results
pub type ResultSender = mpsc::UnboundedSender<MonitoringResult>;
pub type ResultReceiver = mpsc::UnboundedReceiver<MonitoringResult>;

/// Create a result channel
pub fn result_channel() -> (ResultSender, ResultReceiver) {
    mpsc::unbounded_channel()
}
