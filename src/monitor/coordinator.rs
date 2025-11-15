//! Monitoring task coordination and execution

use crate::config::Config;
use crate::core::types::AgentStatus;
use crate::error::Result;
use crate::monitor::PingChecker;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{error, info};

use crate::core::types::MonitoringResult;
use tokio::sync::mpsc;

/// Channel for sending monitoring results
type ResultSender = mpsc::UnboundedSender<MonitoringResult>;
type ResultReceiver = mpsc::UnboundedReceiver<MonitoringResult>;

/// Create a result channel
fn result_channel() -> (ResultSender, ResultReceiver) {
    mpsc::unbounded_channel()
}

/// Run the monitoring loop
pub async fn run_monitoring(
    config: Config,
    agent_status: Arc<RwLock<AgentStatus>>,
    agent_shutdown_rx: &mut broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting monitoring tasks");

    let (result_tx, mut result_rx) = result_channel();

    // Create ping checker
    let ping_checker =
        match PingChecker::new(config.monitoring.timeout(), config.monitoring.ping_count) {
            Ok(checker) => Arc::new(checker),
            Err(e) => {
                error!("Failed to create ping checker: {}", e);
                return Err(e);
            }
        };

    // Spawn monitoring task
    let monitor_handle = {
        let config = config.clone();
        let agent_status = Arc::clone(&agent_status);
        let ping_checker = Arc::clone(&ping_checker);
        let result_tx = result_tx.clone();
        let mut agent_shutdown_rx = agent_shutdown_rx.resubscribe();

        tokio::spawn(async move {
            run_check_loop(
                config,
                agent_status,
                ping_checker,
                result_tx,
                &mut agent_shutdown_rx,
            )
            .await
        })
    };

    // Process results
    let result_handle = {
        let status = Arc::clone(&agent_status);
        let mut agent_shutdown_rx = agent_shutdown_rx.resubscribe();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(result) = result_rx.recv() => {
                        // Update statistics
                        let mut s = status.write();
                        s.checks_performed += 1;
                        if result.success {
                            s.checks_successful += 1;
                        } else {
                            s.checks_failed += 1;
                        }
                    }
                    _ = agent_shutdown_rx.recv() => {
                        info!("Monitoring sub-task shutting down");
                        break;
                    }
                }
            }
        })
    };

    // Wait for shutdown
    // Due to result_handle spawned in another task and resubscribed to shoutdown_rx we need to wait here
    // for the signal as well
    match agent_shutdown_rx.recv().await {
        Ok(_) => tracing::info!("Monitoring task shutting down"),
        Err(_) => tracing::warn!("Monitoring shutdown channel already closed"),
    }

    // Wait for tasks to complete
    let _ = tokio::join!(monitor_handle, result_handle);

    info!("Monitoring tasks stopped");
    Ok(())
}

/// Main check loop that runs periodically
async fn run_check_loop(
    config: Config,
    agent_status: Arc<RwLock<AgentStatus>>,
    ping_checker: Arc<PingChecker>,
    result_tx: ResultSender,
    agent_shutdown_rx: &mut broadcast::Receiver<()>,
) {
    let mut interval = interval(config.monitoring.interval());
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if config.endpoints.is_empty() {
                    continue;
                }

                info!("Running checks for {} endpoints", config.endpoints.len());

                // Run checks concurrently with limit
                let semaphore = Arc::new(tokio::sync::Semaphore::new(config.monitoring.max_concurrent));
                let mut tasks = Vec::new();

                for endpoint in &config.endpoints {
                    let permit = semaphore.clone().acquire_owned().await.unwrap();
                    let ping_checker = Arc::clone(&ping_checker);
                    let agent_id = config.agent_id.clone();
                    let endpoint = endpoint.clone();
                    let result_tx = result_tx.clone();

                    let task = tokio::spawn(async move {
                        let result = ping_checker.check(&agent_id, &endpoint).await;
                        if let Err(e) = result_tx.send(result) {
                            error!("Failed to send result: {}", e);
                        }
                        drop(permit);
                    });

                    tasks.push(task);
                }

                // Wait for all checks to complete
                for task in tasks {
                    let _ = task.await;
                }
            }
            _ = agent_shutdown_rx.recv() => {
                info!("Check_loop shutting down");
                break;
            }
        }
    }
}
