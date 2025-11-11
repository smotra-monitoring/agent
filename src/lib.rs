//! Smotra Agent Library
//!
//! A distributed monitoring agent library for tracking reachability and performance
//! of networked hosts. The library provides:
//!
//! - ICMP ping monitoring
//! - Traceroute capabilities
//! - Configurable monitoring intervals
//! - Data reporting to a central server
//! - Local caching for offline operation
//! - Plugin system for extensibility

pub mod config;
pub mod error;
pub mod monitor;
pub mod plugin;
pub mod reporter;
pub mod types;

use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use tracing::{info, warn};

pub use config::Config;
pub use error::{Error, Result};
pub use types::{MonitoringResult, AgentStatus};

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
        
        Self {
            config: Arc::new(RwLock::new(config)),
            status: Arc::new(RwLock::new(AgentStatus::default())),
            shutdown_tx,
        }
    }

    /// Start the agent and all monitoring tasks
    pub async fn start(&self) -> Result<()> {
        info!("Starting Smotra agent");
        
        let config = self.config.read().clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        
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
                monitor::run_monitoring(config, status, &mut shutdown_rx).await
            })
        };

        // Start reporter task
        let reporter_handle = {
            let config = config.clone();
            let status = Arc::clone(&self.status);
            let mut shutdown_rx = self.shutdown_tx.subscribe();
            
            tokio::spawn(async move {
                reporter::run_reporter(config, status, &mut shutdown_rx).await
            })
        };

        // Wait for shutdown signal
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received");
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl+C received, shutting down");
            }
        }

        // Update status
        {
            let mut status = self.status.write();
            status.is_running = false;
        }

        // Wait for tasks to complete
        let _ = tokio::join!(monitor_handle, reporter_handle);
        
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
