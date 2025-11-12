//! Server reporting functionality

use crate::config::Config;
use crate::core::types::AgentStatus;
use crate::error::{Error, Result};
use chrono::Utc;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Run the reporter loop
pub async fn run_reporter(
    config: Config,
    status: Arc<RwLock<AgentStatus>>,
    shutdown_rx: &mut broadcast::Receiver<()>,
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
                if config.server.is_configured() {
                    match send_report(&config, &status).await {
                        Ok(_) => {
                            let mut s = status.write();
                            s.server_connected = true;
                            s.last_report_at = Some(Utc::now());
                            debug!("Report sent successfully");
                        }
                        Err(e) => {
                            let mut s = status.write();
                            s.server_connected = false;
                            error!("Failed to send report: {}", e);
                        }
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Reporter shutting down");
                break;
            }
        }
    }

    Ok(())
}

/// Send a report to the server
async fn send_report(config: &Config, status: &Arc<RwLock<AgentStatus>>) -> Result<()> {
    let server_url = config
        .server
        .url
        .as_ref()
        .ok_or_else(|| Error::Config("Server URL not configured".to_string()))?;

    let client = reqwest::Client::builder()
        .timeout(config.server.timeout())
        .danger_accept_invalid_certs(!config.server.verify_tls)
        .build()?;

    let status_data = status.read().clone();
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
