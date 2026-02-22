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
    /// Manual trigger (for testing or manual reloads)
    Manual,
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
    let mut file_watcher = ConfigFileWatcher::new(
        config_path.clone(),
        trigger_tx.clone(),
        shutdown_rx.resubscribe(),
    )?;

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
                    ReloadTrigger::FileChange(_) | ReloadTrigger::Signal | ReloadTrigger::Manual => {

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

/// Test helpers for integration tests
///
/// Provides access to internal types and functionality needed for testing.
/// **Internal use only** - API is unstable.
#[doc(hidden)]
pub mod test_helpers {
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
        reload_tx: mpsc::Sender<Config>,
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

                            if let Err(e) = reload_tx.send(new_config).await {
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
    async fn test_reload_trigger_variants() {
        // Test all variants can be created (accessible in tests within the module)
        let _file_trigger = ReloadTrigger::FileChange(PathBuf::from("/tmp/config.toml"));
        let _signal_trigger = ReloadTrigger::Signal;
        let _server_trigger = ReloadTrigger::ServerVersionChange(2);
        let _manual_trigger = ReloadTrigger::Manual;
    }
}
