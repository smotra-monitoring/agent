//! Heartbeat reporting to central server

use crate::agent_config::Config;
use crate::core::{AgentHealthStatus, AgentHeartbeat};
use crate::error::{Error, Result};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

/// Heartbeat reporter for sending lightweight agent status updates
#[derive(Debug)]
pub struct HeartbeatReporter {
    config: Config,
    client: reqwest::Client,
    system: Mutex<System>,
}

impl HeartbeatReporter {
    /// Create a new heartbeat reporter
    pub fn new(config: Config) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.server.timeout())
            .danger_accept_invalid_certs(!config.server.verify_tls)
            .build()?;

        // Initialize system with minimal refresh for better performance
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );

        Ok(Self {
            config,
            client,
            system: Mutex::new(system),
        })
    }

    /// Collect current system metrics for heartbeat
    pub async fn collect_metrics(&self) -> AgentHeartbeat {
        let cpu_usage = self.get_cpu_usage().await;
        let memory_usage = self.get_memory_usage().await;

        let mut heartbeat = AgentHeartbeat::with_metrics(cpu_usage, memory_usage);

        // Determine health status based on metrics
        if let Some(cpu) = cpu_usage {
            if cpu > 90.0 {
                heartbeat = heartbeat.with_status(AgentHealthStatus::Degraded);
            }
        }

        if let Some(mem) = memory_usage {
            // If memory usage is above 90%, consider degraded
            if mem > 90.0 {
                heartbeat = heartbeat.with_status(AgentHealthStatus::Degraded);
            }
        }

        heartbeat
    }

    /// Send heartbeat to the server
    pub async fn send_heartbeat(&self) -> Result<()> {
        let server_url = self
            .config
            .server
            .url
            .as_ref()
            .ok_or_else(|| Error::Config("Server URL not configured".to_string()))?;

        let heartbeat = self.collect_metrics().await;
        let heartbeat_url = format!(
            "{}/api/v1/agent/{}/heartbeat",
            server_url, self.config.agent_id
        );

        debug!(
            "Sending heartbeat to {} for agent {}",
            heartbeat_url, self.config.agent_id
        );

        let mut request = self.client.post(&heartbeat_url).json(&heartbeat);

        // Add agent version header
        request = request.header("X-Agent-Version", self.config.version.to_string());

        // Use Bearer header as specified in OpenAPI spec (AgentApiKey security scheme)
        if let Some(api_key) = &self.config.server.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
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

    /// Get current CPU usage percentage
    async fn get_cpu_usage(&self) -> Option<f32> {
        let mut system = self.system.lock().await;

        // Refresh CPU info
        system.refresh_cpu_all();

        // Get global CPU usage
        let cpu_usage = system.global_cpu_usage();

        if cpu_usage.is_finite() && cpu_usage >= 0.0 {
            Some(cpu_usage)
        } else {
            None
        }
    }

    /// Get current memory usage in MB
    async fn get_memory_usage(&self) -> Option<f32> {
        let mut system = self.system.lock().await;

        // Refresh memory info
        system.refresh_memory();

        // Get total and used memory
        let total_memory = system.total_memory();
        let used_memory = system.used_memory();

        if total_memory > 0 {
            // Calculate percentage
            let memory_percent = (used_memory as f64 / total_memory as f64) * 100.0;
            Some(memory_percent as f32)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_config::{MonitoringConfig, ServerConfig, StorageConfig};
    use chrono::Utc;

    fn create_test_config() -> Config {
        Config {
            version: 1,
            agent_id: "test-agent-001".to_string(),
            agent_name: "Test Agent".to_string(),
            tags: vec!["test".to_string()],
            monitoring: MonitoringConfig::default(),
            server: ServerConfig {
                url: Some("https://test.example.com".to_string()),
                api_key: Some("test-key".to_string()),
                ..Default::default()
            },
            storage: StorageConfig::default(),
            endpoints: vec![],
        }
    }

    #[test]
    fn test_heartbeat_reporter_creation() {
        let config = create_test_config();
        let reporter = HeartbeatReporter::new(config);
        assert!(reporter.is_ok());
    }

    #[tokio::test]
    async fn test_collect_metrics() {
        let config = create_test_config();
        let reporter = HeartbeatReporter::new(config).unwrap();
        let heartbeat = reporter.collect_metrics().await;

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
        let heartbeat = AgentHeartbeat::with_metrics(Some(45.5), Some(512.0));
        let json = serde_json::to_string(&heartbeat).unwrap();

        // Verify JSON contains expected fields
        assert!(json.contains("timestamp"));
        assert!(json.contains("status"));
        assert!(json.contains("cpu_usage_percent"));
        assert!(json.contains("memory_usage_percent"));

        // Verify deserialization works
        let deserialized: AgentHeartbeat = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cpu_usage_percent, Some(45.5));
        assert_eq!(deserialized.memory_usage_percent, Some(512.0));
    }

    #[test]
    fn test_heartbeat_default_status() {
        let heartbeat = AgentHeartbeat::new();
        assert_eq!(heartbeat.status, AgentHealthStatus::Healthy);
    }

    #[test]
    fn test_heartbeat_with_status() {
        let heartbeat = AgentHeartbeat::new().with_status(AgentHealthStatus::Degraded);
        assert_eq!(heartbeat.status, AgentHealthStatus::Degraded);
    }

    #[tokio::test]
    async fn test_system_metrics_collection() {
        let config = create_test_config();
        let reporter = HeartbeatReporter::new(config).unwrap();

        // Get CPU usage
        let cpu = reporter.get_cpu_usage().await;
        if let Some(cpu_val) = cpu {
            assert!(cpu_val >= 0.0, "CPU usage should be non-negative");
            assert!(cpu_val <= 100.0, "CPU usage should not exceed 100%");
        }

        // Get memory usage
        let mem = reporter.get_memory_usage().await;
        if let Some(mem_val) = mem {
            assert!(mem_val > 0.0, "Memory usage should be positive");
            assert!(mem_val <= 100.0, "Memory usage should not exceed 100%");
        }
    }

    #[tokio::test]
    async fn test_metrics_in_heartbeat() {
        let config = create_test_config();
        let reporter = HeartbeatReporter::new(config).unwrap();
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
