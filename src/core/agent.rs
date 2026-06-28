//! Agent implementation module
//!
//! Contains the main Agent struct and its implementation for coordinating
//! monitoring tasks and managing agent lifecycle.

use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};

use super::AgentStatus;
use crate::agent_config::Config;
use crate::cache::ResultCache;
use crate::error::Result;

/// Main agent instance that coordinates all monitoring tasks
pub struct Agent {
    config: Arc<RwLock<Config>>,
    config_path: PathBuf,
    status: Arc<RwLock<AgentStatus>>,
    result_cache: Arc<ResultCache>,
    shutdown_tx: broadcast::Sender<()>,
}

impl Agent {
    /// Create a new agent instance by loading configuration from file
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the configuration file to load
    ///
    /// # Returns
    ///
    /// Returns the agent instance or an error if config loading/validation fails
    pub fn new(config_path: PathBuf) -> Result<Self> {
        // Load and validate configuration from file
        let config = Config::load_and_validate_config(&config_path)?;

        let (shutdown_tx, _) = broadcast::channel(1);
        let mut status = AgentStatus::new();
        status.config_version = config.version as i64;

        let result_cache = Arc::new(ResultCache::new(
            config.storage.max_cached_results,
            std::time::Duration::from_secs(config.storage.max_cache_age_secs),
        ));

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
            status: Arc::new(RwLock::new(status)),
            result_cache,
            shutdown_tx,
        })
    }

    /// Start the agent and all monitoring tasks
    pub async fn start(&self) -> Result<()> {
        let mut shutdown_rx = self.subscribe_shutdown();

        info!("Starting agent id {}", self.config.read().agent_id);

        // Create channel for config hot-reload
        let (reload_config_tx, mut reload_config_rx) = mpsc::channel(1);

        // Update status. Agent is considered "running".
        {
            let mut status = self.status.write();
            status.is_running = true;
            status.started_at = chrono::Utc::now();
        }

        // Start monitoring tasks
        let monitor_handle = {
            let config = Arc::clone(&self.config);
            let status = Arc::clone(&self.status);
            let cache = Arc::clone(&self.result_cache);
            let mut shutdown_rx = self.subscribe_shutdown();

            tokio::spawn(async move {
                crate::monitor::run_monitoring(config, status, cache, &mut shutdown_rx).await
            })
        };

        // Start reporter task
        let reporter_handle = {
            let config = Arc::clone(&self.config);
            let status = Arc::clone(&self.status);
            let mut shutdown_rx = self.subscribe_shutdown();

            tokio::spawn(async move {
                crate::reporter::run_reporter(config, status, &mut shutdown_rx).await
            })
        };

        // Start result-cache reporter task
        let result_reporter_handle = {
            let config = Arc::clone(&self.config);
            let status = Arc::clone(&self.status);
            let cache = Arc::clone(&self.result_cache);
            let shutdown_rx = self.subscribe_shutdown();

            tokio::spawn(async move {
                crate::reporter::run_result_reporter(config, cache, status, shutdown_rx).await
            })
        };

        // Start heartbeat task
        let heartbeat_handle = {
            let config = Arc::clone(&self.config);
            let status = Arc::clone(&self.status);
            let shutdown_rx = self.subscribe_shutdown();

            tokio::spawn(async move {
                crate::reporter::run_heartbeat(config, status, shutdown_rx).await
            })
        };

        // Start updater task
        let updater_handle = {
            let config = Arc::clone(&self.config);
            let shutdown_rx = self.subscribe_shutdown();

            tokio::spawn(async move {
                crate::self_upgrade::run_update_checker(config, shutdown_rx).await
            })
        };

        // Start config hot-reload task
        let hot_reload_handle = {
            let config_path = self.config_path.clone();
            let config_tx = reload_config_tx;
            let shutdown_rx = self.subscribe_shutdown();

            tokio::spawn(async move {
                crate::agent_config::run_hot_reload(config_path, config_tx, shutdown_rx).await
            })
        };

        // Pin a sigterm future that resolves on SIGTERM (Unix) or never (other platforms).
        // We pin it so it can be polled across loop iterations without being recreated.
        let sigterm = wait_sigterm();
        tokio::pin!(sigterm);

        // Wait for shutdown signal or process config reloads
        loop {
            tokio::select! {
                Some(new_config) = reload_config_rx.recv() => {
                    info!("Config reload received from hot reload task");
                    if let Err(e) = self.reload_config(new_config) {
                        error!("Failed to apply reloaded config: {}", e);
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received");
                    break;
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("SIGINT received, shutting down");
                    let _ = self.shutdown_tx.send(());
                    break;
                }
                _ = &mut sigterm => {
                    info!("SIGTERM received, shutting down");
                    let _ = self.shutdown_tx.send(());
                    break;
                }
            }
        }

        info!("Stopping agent");

        // Wait for tasks to complete
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            let _ = monitor_handle.await;
            let _ = reporter_handle.await;
            let _ = result_reporter_handle.await;
            let _ = heartbeat_handle.await;
            let _ = updater_handle.await;
            let _ = hot_reload_handle.await;
        })
        .await
        .ok(); // Ignore timeout error, we just want to wait for tasks to finish if they can

        // Update status.Agent is considered "stopped".
        {
            let mut status = self.status.write();
            status.is_running = false;
            status.stopped_at = Some(chrono::Utc::now());
        }

        info!("Agent stopped");
        Ok(())
    }

    /// Stop the agent gracefully
    pub fn stop(&self) -> Result<()> {
        info!("Stopping agent");
        let _ = self.shutdown_tx.send(());
        Ok(())
    }

    /// Subscribe to shutdown signals
    ///
    /// Returns a broadcast receiver that will receive a signal when the agent is shutting down.
    /// Useful for coordinating shutdown of related tasks like config reload managers.
    pub fn subscribe_shutdown(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Get current agent status
    pub fn status(&self) -> AgentStatus {
        self.status.read().clone()
    }

    /// Hot-reload configuration with validation
    ///
    /// This method validates the new configuration before applying it.
    /// On successful validation, the configuration is updated atomically.
    /// Monitoring tasks will pick up the new configuration on their next cycle.
    ///
    /// # Arguments
    ///
    /// * `new_config` - The new configuration to apply
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if the config was successfully validated and applied,
    /// or an error if validation failed. If validation fails, the old config
    /// remains in effect.
    ///
    /// # Note
    ///
    /// This method does not restart monitoring tasks. They will pick up
    /// the new configuration on their next iteration. For changes that require
    /// immediate effect (like API keys or server URLs), consider restarting the agent.
    pub fn reload_config(&self, new_config: Config) -> Result<()> {
        info!(
            "Attempting to reload configuration (version: {})",
            new_config.version
        );

        // Validate the new configuration
        new_config.validate()?;

        let old_config = self.config_clone();

        self.warn_if_critical_changes(&old_config, &new_config);

        // Apply the new configuration atomically
        *self.config.write() = new_config.clone();
        self.status.write().config_version = new_config.version as i64;

        info!(
            "Configuration reloaded successfully (version: {})",
            new_config.version
        );

        Ok(())
    }

    fn warn_if_critical_changes(&self, old_config: &Config, new_config: &Config) {
        // Check if agent_id changed (this would be problematic)
        if new_config.agent_id != old_config.agent_id {
            warn!(
                "Agent ID changed from {} to {} - this may cause issues with server communication",
                old_config.agent_id, new_config.agent_id
            );
        }

        // Log significant changes
        if new_config.monitoring.interval_secs != old_config.monitoring.interval_secs {
            info!(
                "Monitoring interval changed: {} -> {} seconds",
                old_config.monitoring.interval_secs, new_config.monitoring.interval_secs
            );
        }

        if new_config.endpoints.len() != old_config.endpoints.len() {
            info!(
                "Number of endpoints changed: {} -> {}",
                old_config.endpoints.len(),
                new_config.endpoints.len()
            );
        }

        if new_config.server.url != old_config.server.url {
            info!(
                "Server URL changed: {} -> {}",
                old_config.server.url, new_config.server.url
            );
        }

        if new_config.server.api_key != old_config.server.api_key {
            info!("Server API key was updated");
        }
    }

    /// Get a clone of the current agent configuration.
    pub fn config_clone(&self) -> Config {
        self.config.read().clone()
    }
}

/// Returns a future that resolves when SIGTERM is received (Unix) or never (other platforms).
///
/// By pinning this future before the main `select!` loop, the signal listener is created
/// once and reused across every loop iteration rather than being recreated each time.
#[cfg(unix)]
async fn wait_sigterm() {
    use tokio::signal::unix::{signal, SignalKind};
    match signal(SignalKind::terminate()) {
        Ok(mut stream) => {
            stream.recv().await;
        }
        Err(e) => {
            // If we can't install the handler, log and park forever so the process
            // can still be stopped via SIGINT / shutdown broadcast.
            tracing::warn!("Failed to install SIGTERM handler: {}", e);
            std::future::pending::<()>().await;
        }
    }
}

#[cfg(not(unix))]
async fn wait_sigterm() {
    std::future::pending::<()>().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_reload_config_success() {
        let config = Config {
            agent_id: Uuid::now_v7(),
            agent_name: "Test Agent".to_string(),
            ..Config::default()
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.save_to_file_secure(temp_file.path()).await.unwrap();
        let agent = Agent::new(temp_file.path().to_path_buf()).unwrap();

        // Create a new config with changes
        let mut new_config = config.clone();
        new_config.version = 2;
        new_config.monitoring.interval_secs = 120;
        new_config.agent_name = "Updated Agent".to_string();

        // Reload should succeed
        let result = agent.reload_config(new_config.clone());
        assert!(result.is_ok());

        // Verify the config was updated
        let current_config = agent.config_clone();
        assert_eq!(current_config.version, new_config.version);
        assert_eq!(
            current_config.monitoring.interval_secs,
            new_config.monitoring.interval_secs
        );
        assert_eq!(current_config.agent_name, new_config.agent_name);
        assert_eq!(
            agent.status().config_version,
            new_config.version as i64,
            "config_version in status should be updated after reload"
        );
    }

    #[tokio::test]
    async fn test_reload_config_validation_failure() {
        let original_config = Config {
            agent_id: Uuid::now_v7(),
            ..Config::default()
        };

        let temp_file = NamedTempFile::new().unwrap();
        original_config
            .save_to_file_secure(temp_file.path())
            .await
            .unwrap();
        let agent = Agent::new(temp_file.path().to_path_buf()).unwrap();

        // Create an invalid config (interval_secs = 0)
        let mut invalid_config = original_config.clone();
        invalid_config.monitoring.interval_secs = 0;

        // Reload should fail validation
        let result = agent.reload_config(invalid_config);
        assert!(result.is_err());

        // Verify the old config is still in effect
        let current_config = agent.config_clone();
        assert_eq!(
            current_config.monitoring.interval_secs,
            original_config.monitoring.interval_secs
        );
    }

    #[tokio::test]
    async fn test_reload_config_nil_agent_id() {
        let original_config = Config {
            agent_id: Uuid::now_v7(),
            ..Config::default()
        };

        let temp_file = NamedTempFile::new().unwrap();
        original_config
            .save_to_file_secure(temp_file.path())
            .await
            .unwrap();
        let agent = Agent::new(temp_file.path().to_path_buf()).unwrap();

        // Create a config with nil UUID
        let mut invalid_config = original_config.clone();
        invalid_config.agent_id = Uuid::nil();

        // Reload should fail validation
        let result = agent.reload_config(invalid_config);
        assert!(result.is_err());

        // Verify the old config is still in effect
        let current_config = agent.config_clone();
        assert_eq!(current_config.agent_id, original_config.agent_id);
    }

    #[tokio::test]
    async fn test_update_config() {
        let original_config = Config {
            agent_id: Uuid::now_v7(),
            ..Config::default()
        };

        let temp_file = NamedTempFile::new().unwrap();
        original_config
            .save_to_file_secure(temp_file.path())
            .await
            .unwrap();
        let agent = Agent::new(temp_file.path().to_path_buf()).unwrap();

        // Update config
        let mut new_config = original_config.clone();
        new_config.agent_name = "New Name".to_string();

        let result = agent.reload_config(new_config.clone());
        assert!(result.is_ok());

        // Verify the config was updated
        let current_config = agent.config_clone();
        assert_eq!(current_config.agent_name, new_config.agent_name);
    }
}
