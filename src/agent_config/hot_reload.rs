//! Hot reload orchestration module
//!
//! Coordinates config hot-reload from multiple sources:
//! - File system changes to the config file (via notify debouncer)
//! - SIGHUP signal on Unix systems
//! - Future: Server-initiated config version changes
//!
//! ## Architecture
//!
//! This module follows a channel-based architecture that eliminates callback complexity:
//!
//! ```text
//! ┌─────────────┐
//! │ File Watcher│──┐
//! └─────────────┘  │
//!                  ├──► ReloadTrigger ──► Reload Coordinator ──► Config ──► Agent::start()
//! ┌─────────────┐  │                      (load + validate)        (mpsc)
//! │SIGHUP Handler │──┘
//! └─────────────┘
//! ```
//!
//! Instead of passing an `Arc<Agent>` callback (which would create circular dependencies),
//! the hot reload task loads and validates configs, then sends them through an mpsc channel
//! to the main `Agent::start()` event loop, which applies them via `Agent::reload_config()`.
//!
//! This design:
//! - Decouples hot reload from Agent internals
//! - Eliminates callback overhead
//! - Makes the data flow explicit via channels
//! - Simplifies error handling and shutdown coordination

use std::path::PathBuf;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};

use super::reload::{handle_sighup, ConfigReloadManager};
use crate::agent_config::Config;
use crate::error::Result;

/// Run the hot reload orchestration task
///
/// Coordinates config reloading from multiple sources (file changes, SIGHUP signal).
/// When a reload is triggered, loads and validates the config, then sends it through
/// the provided channel to be applied by the agent.
///
/// # Arguments
///
/// * `config_path` - Path to the configuration file to watch and reload
/// * `reload_tx` - Channel to send validated configs to Agent::start()
/// * `shutdown_rx` - Channel to receive shutdown signals
///
/// # Returns
///
/// Returns Ok(()) when shutdown is complete, or an error if setup fails
///
/// # Architecture
///
/// Spawns three conceptual tasks:
/// 1. **File watcher**: Monitors config file for changes (via ConfigReloadManager's debouncer)
/// 2. **SIGHUP handler**: Listens for SIGHUP signals and sends reload triggers
/// 3. **Reload coordinator**: Main event loop that receives triggers, loads/validates config,
///    and sends validated configs through the channel
pub async fn run_hot_reload(
    config_path: PathBuf,
    reload_tx: mpsc::Sender<Config>,
    shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting config hot-reload orchestration");

    // Create the config reload manager
    let mut reload_manager =
        ConfigReloadManager::new(config_path.clone(), shutdown_rx.resubscribe())?;

    // Start watching for file changes
    if let Err(e) = reload_manager.start_watching_file() {
        warn!("Failed to start config file watching: {}", e);
        warn!("Config hot-reload from file changes will not be available");
    } else {
        info!("Config file watching enabled");
    }

    // Get the reload trigger sender for the SIGHUP handler
    let reload_trigger_tx = reload_manager.reload_sender();

    // Spawn SIGHUP handler task
    let sighup_handle = {
        let shutdown_rx = shutdown_rx.resubscribe();
        tokio::spawn(async move {
            if let Err(e) = handle_sighup(reload_trigger_tx, shutdown_rx).await {
                error!("SIGHUP handler error: {}", e);
            }
        })
    };

    info!("Config hot-reload enabled");

    // Run the main reload coordinator loop
    let result = reload_manager
        .run(move |trigger| {
            let config_path = config_path.clone();
            let reload_tx = reload_tx.clone();
            async move {
                info!("Config reload triggered: {:?}", trigger);

                // Load and validate new config
                match Config::load_and_validate_config(&config_path) {
                    Ok(new_config) => {
                        info!(
                            "Config loaded and validated successfully (version: {})",
                            new_config.version
                        );

                        // Send the validated config to Agent::start() for application
                        if let Err(e) = reload_tx.send(new_config).await {
                            error!("Failed to send reloaded config to agent: {}", e);
                            return Err(crate::error::Error::Config(format!(
                                "Config channel closed: {}",
                                e
                            )));
                        }

                        info!("Config reload completed successfully");
                        Ok(())
                    }
                    Err(e) => {
                        error!("Failed to load config during reload: {}", e);
                        Err(e)
                    }
                }
            }
        })
        .await;

    // Wait for SIGHUP handler to complete (with short timeout)
    let _ = tokio::time::timeout(std::time::Duration::from_millis(500), sighup_handle).await;

    info!("Config hot-reload orchestration stopped");
    result
}

#[cfg(test)]
mod tests {
    use super::super::reload::ReloadTrigger;
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_run_hot_reload_shutdown() {
        // Create a temporary config file
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config {
            agent_id: uuid::Uuid::new_v4(),
            ..Config::default()
        };
        config.save_to_file_secure(temp_file.path()).await.unwrap();

        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let (reload_tx, reload_rx) = mpsc::channel(1);

        // Spawn the hot reload task
        let config_path = temp_file.path().to_path_buf();
        let handle =
            tokio::spawn(async move { run_hot_reload(config_path, reload_tx, shutdown_rx).await });

        // Give it a moment to start
        sleep(Duration::from_millis(50)).await;

        // Send shutdown signal
        let _ = shutdown_tx.send(());

        // Wait for task to complete
        let result = tokio::time::timeout(Duration::from_secs(1), handle).await;
        assert!(
            result.is_ok(),
            "Hot reload task should complete on shutdown"
        );

        // Ensure reload channel is not blocked
        drop(reload_rx);
    }

    #[tokio::test]
    async fn test_run_hot_reload_manual_trigger() {
        // Create a temporary config file
        let temp_file = NamedTempFile::new().unwrap();
        let mut config = Config {
            agent_id: uuid::Uuid::new_v4(),
            ..Config::default()
        };
        config.version = 1;
        config.save_to_file_secure(temp_file.path()).await.unwrap();

        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let (reload_tx, mut reload_rx) = mpsc::channel(1);

        // Create the reload manager to get its trigger sender
        let reload_manager =
            ConfigReloadManager::new(temp_file.path().to_path_buf(), shutdown_rx.resubscribe())
                .unwrap();
        let trigger_tx = reload_manager.reload_sender();

        // Spawn the hot reload task
        let config_path = temp_file.path().to_path_buf();
        let handle = tokio::spawn(async move {
            // Recycle the reload_manager by running it directly
            // (we can't use run_hot_reload because we need the trigger_tx)
            let (reload_tx_inner, mut reload_rx_inner) = mpsc::channel(1);

            // Forward from inner to outer channel
            tokio::spawn(async move {
                while let Some(config) = reload_rx_inner.recv().await {
                    let _ = reload_tx.send(config).await;
                }
            });

            reload_manager
                .run(move |_trigger| {
                    let config_path = config_path.clone();
                    let reload_tx = reload_tx_inner.clone();
                    async move {
                        let new_config = Config::load_and_validate_config(&config_path)?;
                        reload_tx.send(new_config).await.map_err(|e| {
                            crate::error::Error::Config(format!("Channel send failed: {}", e))
                        })?;
                        Ok(())
                    }
                })
                .await
        });

        // Give it a moment to start
        sleep(Duration::from_millis(50)).await;

        // Trigger a manual reload
        let _ = trigger_tx.send(ReloadTrigger::Manual);

        // Wait for the reloaded config
        let received_config = tokio::time::timeout(Duration::from_millis(500), reload_rx.recv())
            .await
            .expect("Should receive config within timeout")
            .expect("Should receive Some(config)");

        assert_eq!(received_config.version, 1);

        // Send shutdown signal
        let _ = shutdown_tx.send(());

        // Wait for task to complete
        let _ = tokio::time::timeout(Duration::from_secs(1), handle).await;
    }
}
