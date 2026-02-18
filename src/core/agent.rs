//! Agent implementation module
//!
//! Contains the main Agent struct and its implementation for coordinating
//! monitoring tasks and managing agent lifecycle.

use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};

use super::AgentStatus;
use crate::agent_config::Config;
use crate::error::Result;

/// Main agent instance that coordinates all monitoring tasks
pub struct Agent {
    config: Arc<RwLock<Config>>,
    status: Arc<RwLock<AgentStatus>>,
    shutdown_tx: broadcast::Sender<()>,
}

impl Agent {
    /// Create a new agent instance with the given configuration
    pub fn new(config: Config) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        let status = AgentStatus::new(config.agent_id);

        Self {
            config: Arc::new(RwLock::new(config)),
            status: Arc::new(RwLock::new(status)),
            shutdown_tx,
        }
    }

    /// Start the agent and all monitoring tasks
    pub async fn start(&self) -> Result<()> {
        let config = self.config.read().clone();
        let mut shutdown_rx = self.subscribe_shutdown();

        info!("Starting agent id {}", config.agent_id);

        // Update status. Agent is considered "running".
        {
            let mut status = self.status.write();
            status.is_running = true;
            status.started_at = Some(chrono::Utc::now());
        }

        // Start monitoring tasks
        let monitor_handle = {
            let config = config.clone();
            let status = Arc::clone(&self.status);
            let mut shutdown_rx = self.subscribe_shutdown();

            tokio::spawn(async move {
                crate::monitor::run_monitoring(config, status, &mut shutdown_rx).await
            })
        };

        // Start reporter task
        let reporter_handle = {
            let config = config.clone();
            let status = Arc::clone(&self.status);
            let mut shutdown_rx = self.subscribe_shutdown();

            tokio::spawn(async move {
                crate::reporter::run_reporter(config, status, &mut shutdown_rx).await
            })
        };

        // Start heartbeat task
        let heartbeat_handle = {
            let config = config.clone();
            let shutdown_rx = self.subscribe_shutdown();

            tokio::spawn(async move { crate::reporter::run_heartbeat(config, shutdown_rx).await })
        };

        // Wait for shutdown signal
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received");
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl+C received, shutting down");
                let _ = self.shutdown_tx.send(());
            }
        }

        info!("Stopping agent");

        // Wait for tasks to complete
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            let _ = monitor_handle.await;
            let _ = reporter_handle.await;
            let _ = heartbeat_handle.await;
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

    /// Update agent configuration
    pub fn update_config(&self, config: Config) -> Result<()> {
        info!("Updating agent configuration");
        *self.config.write() = config;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_agent_creation() {
        let config = Config::default();
        let agent = Agent::new(config);
        assert!(!agent.status().is_running);
    }

    #[test]
    fn test_reload_config_success() {
        let config = Config {
            agent_id: Uuid::new_v4(),
            agent_name: "Test Agent".to_string(),
            ..Config::default()
        };

        let agent = Agent::new(config.clone());

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
    }

    #[test]
    fn test_reload_config_validation_failure() {
        let original_config = Config {
            agent_id: Uuid::new_v4(),
            ..Config::default()
        };

        let agent = Agent::new(original_config.clone());

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

    #[test]
    fn test_reload_config_nil_agent_id() {
        let original_config = Config {
            agent_id: Uuid::new_v4(),
            ..Config::default()
        };

        let agent = Agent::new(original_config.clone());

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

    #[test]
    fn test_update_config() {
        let original_config = Config {
            agent_id: Uuid::new_v4(),
            ..Config::default()
        };

        let agent = Agent::new(original_config.clone());

        // Update config
        let mut new_config = original_config.clone();
        new_config.agent_name = "New Name".to_string();

        let result = agent.update_config(new_config.clone());
        assert!(result.is_ok());

        // Verify the config was updated
        let current_config = agent.config_clone();
        assert_eq!(current_config.agent_name, new_config.agent_name);
    }
}
