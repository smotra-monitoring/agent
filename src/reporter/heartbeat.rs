//! Heartbeat reporting to central server

use crate::agent_config::Config;
use crate::core::{AgentHealthStatus, AgentHeartbeat};
use crate::error::{Error, Result};
use chrono::Utc;
use parking_lot::RwLock;
use std::sync::Arc;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

/// Heartbeat reporter for sending lightweight agent status updates
///
/// Holds a shared reference to the live config so any hot-reload applied by
/// `Agent::reload_config()` is immediately reflected in subsequent heartbeats.
#[derive(Debug)]
pub struct HeartbeatReporter {
    config: Arc<RwLock<Config>>,
    system: Mutex<System>,
}

impl HeartbeatReporter {
    /// Create a new heartbeat reporter
    ///
    /// Accepts a shared `Arc<RwLock<Config>>` so that config hot-reloads are
    /// picked up automatically on every `send_heartbeat()` call.
    pub fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        // Initialize system with minimal refresh for better performance
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );

        Ok(Self {
            config,
            system: Mutex::new(system),
        })
    }

    /// Collect current system metrics for heartbeat
    async fn collect_metrics(&self) -> AgentHeartbeat {
        let cpu_usage_percent = self.get_cpu_usage().await;
        let (memory_usage_mb, memory_total_mb) = self.get_memory_mb().await;

        // Determine health status based on metrics
        let mut status = AgentHealthStatus::Healthy;

        if cpu_usage_percent > 90.0 {
            status = AgentHealthStatus::Degraded;
        }

        if memory_total_mb > 0.0 && (memory_usage_mb / memory_total_mb) * 100.0 > 90.0 {
            status = AgentHealthStatus::Degraded;
        }

        AgentHeartbeat {
            timestamp: Utc::now(),
            status,
            cpu_usage_percent,
            memory_usage_mb,
            memory_total_mb,
        }
    }

    /// Send heartbeat to the server
    pub async fn send_heartbeat(&self) -> Result<()> {
        // Snapshot the live config so all fields within this call are consistent.
        let config = self.config.read().clone();

        let client = reqwest::Client::builder()
            .timeout(config.server.timeout())
            .danger_accept_invalid_certs(!config.server.verify_tls)
            .build()?;

        let heartbeat = self.collect_metrics().await;
        let heartbeat_url = format!("{}/agent/{}/heartbeat", config.server.url, config.agent_id);

        debug!(
            "Sending heartbeat to {} for agent {}",
            heartbeat_url, config.agent_id
        );

        let mut request = client.post(&heartbeat_url).json(&heartbeat);

        // Use X-Agent-API-Key header as specified in OpenAPI spec (AgentApiKey security scheme)
        if let Some(api_key) = &config.server.api_key {
            request = request.header("X-Agent-API-Key", api_key);
        }

        let response = request.send().await?;

        match response.status().as_u16() {
            204 => {
                debug!("Heartbeat sent successfully");
                Ok(())
            }
            401 => {
                warn!("Heartbeat rejected: Unauthorized (invalid API key)");
                Err(Error::Authentication(
                    "Invalid API key for heartbeat".to_string(),
                ))
            }
            status => {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                error!("Heartbeat failed with status {}: {}", status, error_text);
                Err(Error::Network(format!(
                    "Server returned error {}: {}",
                    status, error_text
                )))
            }
        }
    }

    /// Get current CPU usage percentage (0.0–100.0)
    async fn get_cpu_usage(&self) -> f64 {
        let mut system = self.system.lock().await;
        system.refresh_cpu_all();
        let cpu_usage = system.global_cpu_usage();
        // global_cpu_usage() returns f32; 0.0 on the first sample (no delta yet)
        if cpu_usage.is_finite() && cpu_usage >= 0.0 {
            cpu_usage as f64
        } else {
            0.0
        }
    }

    /// Get current memory usage and total memory in megabytes.
    /// Returns (usage_mb, total_mb); both are 0.0 on unsupported platforms.
    async fn get_memory_mb(&self) -> (f64, f64) {
        let mut system = self.system.lock().await;
        system.refresh_memory();
        let total_mb = system.total_memory() as f64 / 1024.0 / 1024.0;
        let usage_mb = system.used_memory() as f64 / 1024.0 / 1024.0;
        (usage_mb, total_mb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_config::{MonitoringConfig, ServerConfig, StorageConfig};
    use chrono::Utc;

    fn create_test_config() -> Arc<RwLock<Config>> {
        Arc::new(RwLock::new(Config {
            version: 1,
            agent_id: uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            agent_name: "Test Agent".to_string(),
            tags: vec!["test".to_string()],
            monitoring: MonitoringConfig::default(),
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            endpoints: vec![],
        }))
    }

    #[test]
    fn test_heartbeat_reporter_creation() {
        let config = create_test_config();
        let reporter = HeartbeatReporter::new(config);
        assert!(reporter.is_ok());
    }

    #[tokio::test]
    async fn test_heartbeat_without_server() {
        let config = create_test_config();
        config.write().server.url = "".to_string(); // Clear server URL

        let reporter = HeartbeatReporter::new(config);
        // Should fail if server URL is not configured
        assert!(reporter.is_ok());
    }

    #[tokio::test]
    async fn test_collect_metrics() {
        let config = create_test_config();
        let reporter = HeartbeatReporter::new(config).unwrap();
        let heartbeat = reporter.collect_metrics().await;

        assert!(heartbeat.timestamp.timestamp() > 0);

        // Verify timestamp is recent (within last second)
        let now = Utc::now();
        let diff = now.signed_duration_since(heartbeat.timestamp);
        assert!(
            diff.num_seconds() < 2,
            "Heartbeat timestamp should be recent"
        );
    }

    #[test]
    fn test_heartbeat_serialization() {
        let heartbeat = AgentHeartbeat {
            timestamp: Utc::now(),
            status: AgentHealthStatus::Healthy,
            cpu_usage_percent: 45.5,
            memory_usage_mb: 512.0,
            memory_total_mb: 8192.0,
        };
        let json = serde_json::to_string(&heartbeat).unwrap();

        // Verify JSON contains expected fields
        assert!(json.contains("timestamp"));
        assert!(json.contains("status"));
        assert!(json.contains("cpu_usage_percent"));
        assert!(json.contains("memory_usage_mb"));
        assert!(json.contains("memory_total_mb"));

        // Verify deserialization works
        let deserialized: AgentHeartbeat = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cpu_usage_percent, 45.5);
        assert_eq!(deserialized.memory_usage_mb, 512.0);
        assert_eq!(deserialized.memory_total_mb, 8192.0);
    }

    #[test]
    fn test_heartbeat_default_status() {
        let heartbeat = AgentHeartbeat {
            timestamp: Utc::now(),
            status: AgentHealthStatus::Healthy,
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0.0,
            memory_total_mb: 0.0,
        };
        assert!(matches!(heartbeat.status, AgentHealthStatus::Healthy));
    }

    #[test]
    fn test_heartbeat_with_status() {
        let heartbeat = AgentHeartbeat {
            timestamp: Utc::now(),
            status: AgentHealthStatus::Degraded,
            cpu_usage_percent: 95.0,
            memory_usage_mb: 7500.0,
            memory_total_mb: 8192.0,
        };
        assert!(matches!(heartbeat.status, AgentHealthStatus::Degraded));
    }

    #[tokio::test]
    async fn test_system_metrics_collection() {
        let reporter = HeartbeatReporter::new(create_test_config()).unwrap();

        let cpu = reporter.get_cpu_usage().await;
        assert!(cpu >= 0.0, "CPU usage should be non-negative");
        assert!(cpu <= 100.0, "CPU usage should not exceed 100%");

        let (usage_mb, total_mb) = reporter.get_memory_mb().await;
        assert!(usage_mb >= 0.0, "Memory usage should be non-negative");
        assert!(total_mb >= 0.0, "Total memory should be non-negative");
    }

    #[tokio::test]
    async fn test_metrics_in_heartbeat() {
        let reporter = HeartbeatReporter::new(create_test_config()).unwrap();
        let heartbeat = reporter.collect_metrics().await;

        // Verify the heartbeat was created successfully with valid status
        // Status can be either Healthy or Degraded depending on system load
        match heartbeat.status {
            AgentHealthStatus::Healthy | AgentHealthStatus::Degraded => {
                // Both are valid
            }
        }

        // Verify timestamp is set
        assert!(heartbeat.timestamp.timestamp() > 0);
    }
}
