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
pub async fn run_reporter(
    config: Config,
    agent_status: Arc<RwLock<AgentStatus>>,
    agent_shutdown_rx: &mut broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting reporter");

    if !config.server.is_configured() {
        warn!("Server not configured, reporter will cache data locally only");
    }

    let mut interval = interval(config.server.report_interval());
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                match config.server.is_configured() {
                    true => {
                        match send_agent_report(&config, &agent_status).await {
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
    let report_url = format!("{}/api/v1/agent/report", server_url);

    let mut request = client.post(&report_url).json(&status_data);

    if let Some(api_key) = &config.server.api_key {
        request = request.header("Authorization", format!("Bearer {}", api_key));
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
pub async fn run_heartbeat(
    config: Config,
    mut agent_shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting heartbeat reporter");

    if !config.server.is_configured() {
        warn!("Server not configured, heartbeat disabled");
        return Ok(());
    }

    let heartbeat_reporter = HeartbeatReporter::new(config.clone())?;
    let mut interval = interval(config.server.heartbeat_interval());
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
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
