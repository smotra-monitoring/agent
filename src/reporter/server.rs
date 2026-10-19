//! Server reporting functionality

use crate::core::AgentStatus;
use crate::error::{Error, Result};
use crate::openapi::AgentConfig;
use crate::reporter::HeartbeatReporter;
use chrono::Utc;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Run the heartbeat loop in a separate task
///
/// Accepts a shared `Arc<RwLock<Config>>` so that config hot-reloads are
/// reflected in subsequent heartbeat payloads automatically.
/// Accepts a shared `Arc<RwLock<AgentStatus>>` so each heartbeat payload
/// includes the latest agent status snapshot.
pub async fn run_heartbeat(
    config: Arc<RwLock<AgentConfig>>,
    agent_status: Arc<RwLock<AgentStatus>>,
    mut agent_shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting heartbeat reporter");

    if !config.read().server.is_configured() {
        warn!("Server not configured, heartbeat disabled");
        return Ok(());
    }

    let heartbeat_reporter =
        HeartbeatReporter::new(Arc::clone(&config), Arc::clone(&agent_status))?;

    // Track the current interval so we can hot-reload it when config changes.
    let mut current_interval_duration = config.read().server.heartbeat_interval();
    let mut iv = interval(current_interval_duration);
    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = iv.tick() => {
                // Detect interval changes and recreate the timer.
                let new_interval = config.read().server.heartbeat_interval();
                if new_interval != current_interval_duration {
                    info!(
                        "Heartbeat interval changed from {:?} to {:?}, recreating timer",
                        current_interval_duration, new_interval
                    );
                    current_interval_duration = new_interval;
                    iv = interval(current_interval_duration);
                    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                }

                match heartbeat_reporter.send_heartbeat().await {
                    Ok(_) => {
                        debug!("Heartbeat sent successfully");
                        agent_status.write().server_connected = true;
                        agent_status.write().reported_at = Utc::now();
                    }
                    Err(e) => {
                        agent_status.write().server_connected = false;
                        agent_status.write().failed_report_count += 1;

                        // Log error but continue - heartbeats are best-effort
                        match &e {
                            Error::Authentication(_) => {
                                error!("Heartbeat authentication failed: {}", e);
                            }
                            Error::Network(_) => {
                                warn!("Heartbeat network error: {}", e);
                            }
                            _ => {
                                error!("Heartbeat failed: {}", e);
                            }
                        }
                    }
                }
            }
            _ = agent_shutdown_rx.recv() => {
                info!("Heartbeat reporter shutting down");
                break;
            }
        }
    }

    Ok(())
}
