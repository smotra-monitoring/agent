//! Server reporting functionality

use crate::agent_config::Config;
use crate::core::AgentStatus;
use crate::error::{Error, Result};
use crate::reporter::HeartbeatReporter;
use chrono::Utc;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Run the reporter loop
///
/// Accepts a shared `Arc<RwLock<Config>>` so that config hot-reloads applied by
/// `Agent::reload_config()` are picked up on every reporting tick.
pub async fn run_reporter(
    config: Arc<RwLock<Config>>,
    agent_status: Arc<RwLock<AgentStatus>>,
    agent_shutdown_rx: &mut broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting reporter");

    if !config.read().server.is_configured() {
        warn!("Server not configured, reporter will cache data locally only");
    }

    // Track the current interval so we can hot-reload it when config changes.
    let mut current_interval_duration = config.read().server.report_interval();
    let mut iv = interval(current_interval_duration);
    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = iv.tick() => {
                // Take a consistent snapshot for this tick.
                let config_snapshot = config.read().clone();

                // Detect interval changes and recreate the timer.
                let new_interval = config_snapshot.server.report_interval();
                if new_interval != current_interval_duration {
                    info!(
                        "Report interval changed from {:?} to {:?}, recreating timer",
                        current_interval_duration, new_interval
                    );
                    current_interval_duration = new_interval;
                    iv = interval(current_interval_duration);
                    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                }

                match config_snapshot.server.is_configured() {
                    true => {
                        match send_agent_report(&config_snapshot, &agent_status).await {
                            Ok(_) => {
                                let mut s = agent_status.write();
                                s.server_connected = true;
                                s.last_report_at = Some(Utc::now());
                                debug!("Report sent successfully");
                            }
                            Err(e) => {
                                let mut s = agent_status.write();
                                s.server_connected = false;
                                s.failed_report_count += 1;
                                error!("Failed to send report: {}", e);
                            }
                        }
                    }
                    false => {
                        warn!("Implement local caching logic");
                    }
                }
            }
            _ = agent_shutdown_rx.recv() => {
                info!("Agent status reporter shutting down");
                break;
            }
        }
    }

    Ok(())
}

/// Send an agent report to the server
async fn send_agent_report(config: &Config, agent_status: &Arc<RwLock<AgentStatus>>) -> Result<()> {
    let server_url = &config.server.url;

    let client = reqwest::Client::builder()
        .timeout(config.server.timeout())
        .danger_accept_invalid_certs(!config.server.verify_tls)
        .build()?;

    let status_data = agent_status.read().clone();
    let report_url = format!("{}/agent/report", server_url);

    let mut request = client.post(&report_url).json(&status_data);

    if let Some(api_key) = &config.server.api_key {
        request = request.header("X-Agent-API-Key", api_key);
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        return Err(Error::Network(format!(
            "Server returned error: {}",
            response.status()
        )));
    }

    Ok(())
}

/// Run the heartbeat loop in a separate task
///
/// Accepts a shared `Arc<RwLock<Config>>` so that config hot-reloads are
/// reflected in subsequent heartbeat payloads automatically.
pub async fn run_heartbeat(
    config: Arc<RwLock<Config>>,
    mut agent_shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting heartbeat reporter");

    if !config.read().server.is_configured() {
        warn!("Server not configured, heartbeat disabled");
        return Ok(());
    }

    let heartbeat_reporter = HeartbeatReporter::new(Arc::clone(&config))?;

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
                    }
                    Err(e) => {
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
