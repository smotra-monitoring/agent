//! Monitoring task coordination and execution

use crate::agent_config::Config;
use crate::cache::ResultCache;
use crate::core::AgentStatus;
use crate::error::Result;
use crate::monitor::PingChecker;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{debug, error, info};

use crate::core::MonitoringResult;
use tokio::sync::mpsc;

/// Channel for sending monitoring results
type ResultSender = mpsc::UnboundedSender<MonitoringResult>;

/// Run the monitoring loop
///
/// Accepts a shared `Arc<RwLock<Config>>` so that config hot-reloads applied by
/// `Agent::reload_config()` are picked up on every monitoring tick.
pub async fn run_monitoring(
    agent_config: Arc<RwLock<Config>>,
    agent_status: Arc<RwLock<AgentStatus>>,
    result_cache: Arc<ResultCache>,
    agent_shutdown_rx: &mut broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting monitoring tasks");

    let (result_tx, result_rx) = mpsc::unbounded_channel::<MonitoringResult>();

    // Spawn monitoring task
    let monitor_handle = {
        let config = Arc::clone(&agent_config);
        let result_tx = result_tx.clone();

        let mut agent_shutdown_rx = agent_shutdown_rx.resubscribe();

        tokio::spawn(async move { run_check_loop(config, result_tx, &mut agent_shutdown_rx).await })
    };

    // Process results
    let result_handle = {
        let agent_status = Arc::clone(&agent_status);
        let result_cache = Arc::clone(&result_cache);
        let mut agent_shutdown_rx = agent_shutdown_rx.resubscribe();

        tokio::spawn(async move {
            result_collect_loop(
                agent_status,
                result_cache,
                result_rx,
                &mut agent_shutdown_rx,
            )
            .await;
        })
    };

    // Wait for shutdown
    // Due to result_handle spawned in another task and resubscribed to shutdown_rx we need to wait here
    // for the signal as well
    match agent_shutdown_rx.recv().await {
        Ok(_) => tracing::info!("Monitoring task shutting down"),
        Err(_) => tracing::warn!("Monitoring shutdown channel already closed"),
    }

    // Wait for tasks to complete
    let _ = tokio::join!(monitor_handle, result_handle);

    info!("Monitoring and result collection tasks stopped");
    Ok(())
}

async fn result_collect_loop(
    agent_status: Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, AgentStatus>>,
    result_cache: Arc<ResultCache>,
    mut result_rx: mpsc::UnboundedReceiver<MonitoringResult>,
    agent_shutdown_rx: &mut broadcast::Receiver<()>,
) {
    loop {
        tokio::select! {
            Some(result) = result_rx.recv() => {
                // Update statistics
                {
                    let mut s = agent_status.write();
                    s.checks_performed += 1;
                    if result.is_successful() {
                        s.checks_successful += 1;
                    } else {
                        s.checks_failed += 1;
                    }
                }
                result_cache.push(result).await;
                // Reflect current cache depth in agent status (after push to avoid off-by-one)
                let stats = result_cache.stats().await;
                {
                    let mut s = agent_status.write();
                    s.cache_stats.len = stats.len as i64;
                    s.cache_stats.capacity = stats.capacity as i64;
                }
            }
            _ = agent_shutdown_rx.recv() => {
                info!("Monitoring coordinator shutting down");
                break;
            }
        }
    }
}

/// Main check loop that runs periodically
///
/// Reads a fresh config snapshot on every tick so that hot-reloaded values
/// (endpoints, intervals, ping parameters) take effect without a restart.
async fn run_check_loop(
    agent_config: Arc<RwLock<Config>>,
    result_tx: ResultSender,
    agent_shutdown_rx: &mut broadcast::Receiver<()>,
) {
    // Bootstrap the interval from the current config.
    let mut current_interval_duration = agent_config.read().monitoring.interval();
    let mut iv = interval(current_interval_duration);
    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = iv.tick() => {
                // Take a consistent snapshot for this tick.
                let config = agent_config.read().clone();

                // Detect interval changes and recreate the timer.
                let new_interval = config.monitoring.interval();
                if new_interval != current_interval_duration {
                    info!(
                        "Monitoring interval changed from {:?} to {:?}, recreating timer",
                        current_interval_duration, new_interval
                    );
                    current_interval_duration = new_interval;
                    iv = interval(current_interval_duration);
                    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                }

                // Recreate PingChecker from current config snapshot so that
                // timeout and ping_count changes take effect immediately.
                let ping_checker = match PingChecker::new(
                    config.monitoring.timeout(),
                    config.monitoring.ping_count,
                ) {
                    Ok(checker) => Arc::new(checker),
                    Err(e) => {
                        error!("Failed to create ping checker: {}", e);
                        continue;
                    }
                };

                // Filter only enabled endpoints
                let enabled_endpoints: Vec<_> = config.endpoints.iter()
                    .filter(|e| e.enabled)
                    .cloned()
                    .collect();

                if enabled_endpoints.is_empty() {
                    continue;
                }

                debug!("Running checks for {} enabled endpoints", enabled_endpoints.len());

                // Run checks concurrently with limit
                let semaphore = Arc::new(tokio::sync::Semaphore::new(config.monitoring.max_concurrent));
                let mut tasks = Vec::new();

                for endpoint in enabled_endpoints {
                    let permit = semaphore.clone().acquire_owned().await.unwrap();
                    let ping_checker = Arc::clone(&ping_checker);
                    let agent_id = config.agent_id;
                    let result_tx = result_tx.clone();

                    let task = tokio::spawn(async move {
                        let result = ping_checker.check(agent_id, &endpoint).await;
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
