//! Agent implementation module
//!
//! Contains the main Agent struct and its implementation for coordinating
//! monitoring tasks and managing agent lifecycle.

use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

use super::AgentStatus;
use crate::config::Config;
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

        let agent_id = &config.agent_id;
        let status = AgentStatus::new(agent_id);

        Self {
            config: Arc::new(RwLock::new(config)),
            status: Arc::new(RwLock::new(status)),
            shutdown_tx,
        }
    }

    /// Start the agent and all monitoring tasks
    pub async fn start(&self) -> Result<()> {
        let config = self.config.read().clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        info!("Starting agent id {}", config.agent_id);

        // Update status
        {
            let mut status = self.status.write();
            status.is_running = true;
            status.started_at = Some(chrono::Utc::now());
        }

        // Start monitoring tasks
        let monitor_handle = {
            let config = config.clone();
            let status = Arc::clone(&self.status);
            let mut shutdown_rx = self.shutdown_tx.subscribe();

            tokio::spawn(async move {
                crate::monitor::run_monitoring(config, status, &mut shutdown_rx).await
            })
        };

        // Start reporter task
        let reporter_handle = {
            let config = config.clone();
            let status = Arc::clone(&self.status);
            let mut shutdown_rx = self.shutdown_tx.subscribe();

            tokio::spawn(async move {
                crate::reporter::run_reporter(config, status, &mut shutdown_rx).await
            })
        };

        // Start heartbeat task
        let heartbeat_handle = {
            let config = config.clone();
            let shutdown_rx = self.shutdown_tx.subscribe();

            tokio::spawn(async move {
                crate::reporter::run_heartbeat(config, shutdown_rx).await
            })
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
        let _ = tokio::join!(monitor_handle, reporter_handle, heartbeat_handle);

        // Update status
        self.status.write().is_running = false;
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

    /// Get current configuration
    pub fn config(&self) -> Config {
        self.config.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let config = Config::default();
        let agent = Agent::new(config);
        assert!(!agent.status().is_running);
    }
}
