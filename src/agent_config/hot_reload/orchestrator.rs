//! Hot reload orchestration
//!
//! Coordinates config hot-reload from multiple sources:
//! - File system changes to the config file (via notify debouncer)
//! - SIGHUP signal on Unix systems
//! - Future: Server-initiated config version changes

use std::path::PathBuf;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};

use super::config_file_watcher::ConfigFileWatcher;
use super::sighup::handle_sighup;
use crate::agent_config::Config;
use crate::error::Result;

/// Events that trigger config reload
///
/// **Internal use only** - Used by test helpers for integration testing.
/// Not part of the public API - API is unstable and may change.
#[derive(Debug, Clone)]
#[doc(hidden)]
pub enum ReloadTrigger {
    /// Config file was modified on filesystem
    FileChange(PathBuf),
    /// SIGHUP signal received (Unix only)
    Signal,
    /// Server reported a new config version (future implementation)
    ServerVersionChange(u32),
}

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
/// Creates three coordinated components:
/// 1. **File watcher**: Monitors config file for changes (ConfigFileWatcher)
/// 2. **SIGHUP handler**: Listens for SIGHUP signals and sends reload triggers
/// 3. **Main event loop**: Receives triggers from internal channel, loads/validates config,
///    and sends validated configs to Agent through the provided channel
pub async fn run_hot_reload(
    config_path: PathBuf,
    config_tx: mpsc::Sender<Config>,
    shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting config hot-reload orchestration");

    // Create internal channel for reload triggers
    let (trigger_tx, mut trigger_rx) = mpsc::unbounded_channel::<ReloadTrigger>();

    // Create the config file watcher
    let mut file_watcher = ConfigFileWatcher::new(config_path.clone(), trigger_tx.clone())?;

    // Start watching for file changes
    if let Err(e) = file_watcher.start_watching() {
        warn!("Failed to start config file watching: {}", e);
        warn!("Config hot-reload from file changes will not be available");
    } else {
        info!("Config file watching enabled");
    }

    // Spawn SIGHUP handler task
    let sighup_handle = {
        let trigger_tx = trigger_tx.clone();
        let shutdown_rx = shutdown_rx.resubscribe();
        tokio::spawn(async move {
            if let Err(e) = handle_sighup(trigger_tx, shutdown_rx).await {
                error!("SIGHUP handler error: {}", e);
            }
        })
    };

    info!("Config hot-reload enabled");

    // Run the main reload coordinator loop
    let mut shutdown_rx = shutdown_rx;
    loop {
        tokio::select! {
            Some(trigger) = trigger_rx.recv() => {
                info!("Config reload triggered: {:?}", trigger);

                match trigger {
                    ReloadTrigger::FileChange(_) | ReloadTrigger::Signal
                     => {

                        match Config::load_and_validate_config(&config_path) {
                            Ok(new_config) => {
                                info!(
                                    "Config loaded and validated successfully (version: {})",
                                    new_config.version
                                );

                                // Send the validated config to Agent::start() for application
                                if let Err(e) = config_tx.send(new_config).await {
                                    error!("Failed to send config to a closed channel: {}", e);
                                    // Channel closed, break the loop
                                    break;
                                }

                                info!("Config reload completed successfully");
                            }
                            Err(e) => {
                                error!("Failed to load config during reload: {}", e);
                                // Continue running even if one reload fails
                            }
                        }

                    }
                    ReloadTrigger::ServerVersionChange(version) => {
                        info!("Reload triggered by server version change: {}", version);
                        unimplemented!("Server-initiated config version change handling is not implemented yet");
                    }
                }

            }
            _ = shutdown_rx.recv() => {
                info!("Config hot-reload orchestration shutting down");
                break;
            }
        }
    }

    // Keep file_watcher alive until shutdown (it owns the Debouncer)
    drop(file_watcher);

    // Wait for SIGHUP handler to complete (with short timeout)
    let _ = tokio::time::timeout(std::time::Duration::from_millis(500), sighup_handle).await;

    info!("Config hot-reload orchestration stopped");
    Ok(())
}

/// Test helpers for unit tests
///
/// Provides access to internal types and functionality needed for testing.
/// **Internal use only** - API is unstable.
#[cfg(test)]
mod test_helpers {
    use super::*;

    // Re-export ReloadTrigger for test use
    pub use super::ReloadTrigger;

    /// Create a test-only reload trigger sender
    ///
    /// Returns a channel sender that can be used to manually trigger reloads for testing
    pub fn create_reload_trigger_channel() -> (
        mpsc::UnboundedSender<ReloadTrigger>,
        mpsc::UnboundedReceiver<ReloadTrigger>,
    ) {
        mpsc::unbounded_channel()
    }

    /// Run hot reload orchestration with custom trigger channel for testing
    ///
    /// This allows tests to manually send reload triggers
    pub async fn run_hot_reload_with_trigger_channel(
        config_path: PathBuf,
        config_tx: mpsc::Sender<Config>,
        mut trigger_rx: mpsc::UnboundedReceiver<ReloadTrigger>,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<()> {
        info!("Starting test hot-reload orchestration with custom trigger channel");

        let mut shutdown_rx = shutdown_rx;
        loop {
            tokio::select! {
                Some(trigger) = trigger_rx.recv() => {
                    info!("Config reload triggered: {:?}", trigger);

                    match Config::load_and_validate_config(&config_path) {
                        Ok(new_config) => {
                            info!(
                                "Config loaded and validated successfully (version: {})",
                                new_config.version
                            );

                            if let Err(e) = config_tx.send(new_config).await {
                                error!("Failed to send reloaded config: {}", e);
                                break;
                            }

                            info!("Config reload completed successfully");
                        }
                        Err(e) => {
                            error!("Failed to load config during reload: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Test hot-reload orchestration shutting down");
                    break;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Agent;
    use crate::{Endpoint, MonitoringConfig};
    use std::fs;
    use tempfile::NamedTempFile;
    use tokio::time::{sleep, Duration};

    /// Helper to create a valid config file content
    fn config_with_endpoints(version: u32, interval_secs: u64, endpoints_count: usize) -> Config {
        let endpoints: Vec<Endpoint> = (1..=endpoints_count)
            .map(|i| Endpoint::new(format!("192.168.1.{}", i)))
            .collect();

        Config {
            version,
            agent_id: uuid::Uuid::new_v4(),
            monitoring: MonitoringConfig {
                interval_secs,
                ..MonitoringConfig::default()
            },
            endpoints,
            ..Config::default()
        }
    }

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
        let (config_tx, config_rx) = mpsc::channel(1);

        // Spawn the hot reload task
        let config_path = temp_file.path().to_path_buf();
        let handle =
            tokio::spawn(async move { run_hot_reload(config_path, config_tx, shutdown_rx).await });

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
        drop(config_rx);
    }

    #[tokio::test]
    async fn test_config_reload_manual_trigger() {
        // Create config files
        let config_path = NamedTempFile::new().unwrap();
        let initial_config = config_with_endpoints(1, 60, 2);
        initial_config
            .save_to_file_secure(&config_path)
            .await
            .unwrap();

        let agent = Agent::new(config_path.path().to_path_buf()).unwrap();

        // Override with a different config file for reload testing
        let config_path2 = NamedTempFile::new().unwrap();
        let updated_config = config_with_endpoints(5, 90, 4);
        updated_config
            .save_to_file_secure(&config_path2)
            .await
            .unwrap();

        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let (trigger_tx, trigger_rx) = test_helpers::create_reload_trigger_channel();
        let (config_tx, mut config_rx) = mpsc::channel(1);

        // Spawn the test reload handler
        let config_path2_clone = config_path2.path().to_path_buf();
        let reload_handle = tokio::spawn(async move {
            test_helpers::run_hot_reload_with_trigger_channel(
                config_path2_clone,
                config_tx,
                trigger_rx,
                shutdown_rx,
            )
            .await
        });

        // Manually trigger reload
        trigger_tx.send(ReloadTrigger::Signal).unwrap();

        // Wait for the reloaded config
        let new_config = tokio::time::timeout(Duration::from_millis(500), config_rx.recv())
            .await
            .expect("Should receive config within timeout")
            .expect("Should receive Some(config)");

        // Apply the config to agent
        agent.reload_config(new_config).unwrap();

        // Verify
        let current_config = agent.config_clone();
        assert_eq!(current_config.version, 5);
        assert_eq!(current_config.monitoring.interval_secs, 90);
        assert_eq!(current_config.endpoints.len(), 4);

        // Shutdown
        let _ = shutdown_tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(1), reload_handle)
            .await
            .expect("Hot reload task should shutdown cleanly");
    }

    #[tokio::test]
    async fn test_config_reload_invalid_config() {
        // Create config file
        let config_path = NamedTempFile::new().unwrap();
        let initial_config = config_with_endpoints(1, 60, 2);
        initial_config
            .save_to_file_secure(&config_path)
            .await
            .unwrap();

        let agent = Agent::new(config_path.path().to_path_buf()).unwrap();

        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let (trigger_tx, trigger_rx) = test_helpers::create_reload_trigger_channel();
        let (config_tx, mut config_rx) = mpsc::channel(1);

        // Spawn reload handler
        let config_path_clone = config_path.path().to_path_buf();
        let reload_handle = tokio::spawn(async move {
            test_helpers::run_hot_reload_with_trigger_channel(
                config_path_clone,
                config_tx,
                trigger_rx,
                shutdown_rx,
            )
            .await
        });

        // Write invalid config (interval_secs = 0)
        let invalid_config = config_with_endpoints(2, 0, 1); // 0 is invalid
        invalid_config
            .save_to_file_secure(&config_path)
            .await
            .unwrap();

        // Trigger reload
        trigger_tx.send(ReloadTrigger::Signal).unwrap();

        // Try to receive new config - should not get one within timeout because validation fails
        let result = tokio::time::timeout(Duration::from_millis(200), config_rx.recv()).await;

        assert!(
            result.is_err(),
            "Timeout should occur because invalid config should not be sent through the channel"
        );

        // Verify old config is still in effect (no reload happened)
        let current_config = agent.config_clone();
        assert_eq!(current_config.version, 1);
        assert_eq!(current_config.monitoring.interval_secs, 60);

        // Shutdown
        let _ = shutdown_tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(1), reload_handle)
            .await
            .expect("Hot reload task should shutdown cleanly");
    }

    #[tokio::test]
    async fn test_config_reload_malformed_toml() {
        // Create config file
        let config_path = NamedTempFile::new().unwrap();
        let initial_config = config_with_endpoints(1, 60, 1);
        initial_config
            .save_to_file_secure(&config_path)
            .await
            .unwrap();

        let agent = Agent::new(config_path.path().to_path_buf()).unwrap();

        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let (trigger_tx, trigger_rx) = test_helpers::create_reload_trigger_channel();
        let (config_tx, mut config_rx) = mpsc::channel(1);

        // Spawn reload handler
        let config_path_clone = config_path.path().to_path_buf();
        let reload_handle = tokio::spawn(async move {
            test_helpers::run_hot_reload_with_trigger_channel(
                config_path_clone,
                config_tx,
                trigger_rx,
                shutdown_rx,
            )
            .await
        });

        // Write malformed TOML
        fs::write(&config_path, "invalid [ toml content").unwrap();

        // Trigger reload
        trigger_tx.send(ReloadTrigger::Signal).unwrap();

        // Should not receive new config because parsing fails
        let result = tokio::time::timeout(Duration::from_millis(200), config_rx.recv()).await;

        assert!(
            result.is_err(),
            "Timeout should occur because malformed config should not be sent through channel"
        );

        // Verify old config is still in effect
        let current_config = agent.config_clone();
        assert_eq!(current_config.version, 1);
        assert_eq!(current_config.monitoring.interval_secs, 60);

        // Shutdown
        let _ = shutdown_tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(1), reload_handle).await;
    }

    #[tokio::test]
    async fn test_multiple_config_reloads() {
        // Create config file
        let config_path = NamedTempFile::new().unwrap();
        let initial_config = config_with_endpoints(1, 60, 2);
        initial_config
            .save_to_file_secure(&config_path)
            .await
            .unwrap();

        let agent = Agent::new(config_path.path().to_path_buf()).unwrap();
        let agent_arc = std::sync::Arc::new(agent);

        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let (trigger_tx, trigger_rx) = test_helpers::create_reload_trigger_channel();
        let (config_tx, mut config_rx) = mpsc::channel(1);

        // Spawn reload handler
        let config_path_clone = config_path.path().to_path_buf();
        let agent_for_task = agent_arc.clone();
        let reload_handle = tokio::spawn(async move {
            let reload_task = tokio::spawn(async move {
                test_helpers::run_hot_reload_with_trigger_channel(
                    config_path_clone,
                    config_tx,
                    trigger_rx,
                    shutdown_rx,
                )
                .await
            });

            // Apply configs as they come through the channel
            while let Some(new_config) = config_rx.recv().await {
                let _ = agent_for_task.reload_config(new_config);
            }

            let _ = reload_task.await;
        });

        // Perform multiple reloads
        for i in 2..=5 {
            let updated_config = config_with_endpoints(i, 60 + (i as u64 * 10), i as usize);
            updated_config
                .save_to_file_secure(&config_path)
                .await
                .unwrap();

            // Trigger manual reload
            trigger_tx.send(ReloadTrigger::Signal).unwrap();

            // Wait for reload to be applied
            tokio::time::sleep(Duration::from_millis(100)).await;

            let current_config = agent_arc.config_clone();
            assert_eq!(current_config.version, i);
            assert_eq!(
                current_config.monitoring.interval_secs,
                60 + (i as u64 * 10)
            );
        }

        // Shutdown
        let _ = shutdown_tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(1), reload_handle).await;
    }

    #[tokio::test]
    async fn test_reload_trigger_variants() {
        // Create config file
        let config_path = NamedTempFile::new().unwrap();
        let initial_config = config_with_endpoints(1, 60, 1);
        initial_config
            .save_to_file_secure(&config_path)
            .await
            .unwrap();

        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let (trigger_tx, trigger_rx) = test_helpers::create_reload_trigger_channel();
        let (config_tx, mut config_rx) = mpsc::channel(1);

        // Spawn reload handler
        let config_path_clone = config_path.path().to_path_buf();
        let reload_handle = tokio::spawn(async move {
            test_helpers::run_hot_reload_with_trigger_channel(
                config_path_clone,
                config_tx,
                trigger_rx,
                shutdown_rx,
            )
            .await
        });

        // Test different trigger types - all should work
        trigger_tx
            .send(ReloadTrigger::FileChange(config_path.path().to_path_buf()))
            .unwrap();
        let config = tokio::time::timeout(Duration::from_millis(100), config_rx.recv())
            .await
            .expect("Should receive reload trigger within timeout");

        assert!(config.is_some(), "Expected config received");

        trigger_tx.send(ReloadTrigger::Signal).unwrap();
        let config = tokio::time::timeout(Duration::from_millis(100), config_rx.recv())
            .await
            .expect("Should receive reload trigger within timeout");

        assert!(config.is_some(), "Expected config received");

        trigger_tx
            .send(ReloadTrigger::ServerVersionChange(3))
            .unwrap();
        let config = tokio::time::timeout(Duration::from_millis(100), config_rx.recv())
            .await
            .expect("Should receive reload trigger within timeout");

        assert!(config.is_some(), "Expected config received");

        // Shutdown
        let _ = shutdown_tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(1), reload_handle).await;
    }
}
