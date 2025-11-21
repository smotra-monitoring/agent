//! Heartbeat reporting to central server

use crate::config::Config;
use crate::core::{AgentHealthStatus, AgentHeartbeat};
use crate::error::{Error, Result};
use tracing::{debug, error, warn};

/// Heartbeat reporter for sending lightweight agent status updates
#[derive(Debug)]
pub struct HeartbeatReporter {
    config: Config,
    client: reqwest::Client,
}

impl HeartbeatReporter {
    /// Create a new heartbeat reporter
    pub fn new(config: Config) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.server.timeout())
            .danger_accept_invalid_certs(!config.server.verify_tls)
            .build()?;

        Ok(Self { config, client })
    }

    /// Collect current system metrics for heartbeat
    pub fn collect_metrics(&self) -> AgentHeartbeat {
        // For now, we'll create a basic heartbeat without system metrics
        // System metrics collection can be added later using a crate like sysinfo
        let cpu_usage = Self::get_cpu_usage();
        let memory_usage = Self::get_memory_usage();

        let mut heartbeat = AgentHeartbeat::with_metrics(cpu_usage, memory_usage);

        // Determine health status based on metrics
        if let Some(cpu) = cpu_usage {
            if cpu > 90.0 {
                heartbeat = heartbeat.with_status(AgentHealthStatus::Degraded);
            }
        }

        if let Some(mem) = memory_usage {
            // If memory usage is above 2GB, consider degraded (arbitrary threshold)
            if mem > 2048.0 {
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

        let heartbeat = self.collect_metrics();
        let heartbeat_url = format!(
            "{}/api/v1/agent/{}/heartbeat",
            server_url, self.config.agent_id
        );

        debug!(
            "Sending heartbeat to {} for agent {}",
            heartbeat_url, self.config.agent_id
        );

        let mut request = self.client.post(&heartbeat_url).json(&heartbeat);

        // Use X-API-Key header as specified in OpenAPI spec (AgentApiKey security scheme)
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
    /// TODO: Implement actual CPU usage collection using sysinfo crate
    fn get_cpu_usage() -> Option<f32> {
        // Placeholder - would use sysinfo crate in production
        None
    }

    /// Get current memory usage in MB
    /// TODO: Implement actual memory usage collection using sysinfo crate
    fn get_memory_usage() -> Option<f32> {
        // Placeholder - would use sysinfo crate in production
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MonitoringConfig, ServerConfig, StorageConfig};
    use chrono::Utc;

    fn create_test_config() -> Config {
        Config {
            agent_id: "test-agent-001".to_string(),
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

    #[test]
    fn test_collect_metrics() {
        let config = create_test_config();
        let reporter = HeartbeatReporter::new(config).unwrap();
        let heartbeat = reporter.collect_metrics();

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
        assert!(json.contains("memory_usage_mb"));

        // Verify deserialization works
        let deserialized: AgentHeartbeat = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cpu_usage_percent, Some(45.5));
        assert_eq!(deserialized.memory_usage_mb, Some(512.0));
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
}
