//! Common types used throughout the agent

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Re-export from generated OpenAPI types — these are the canonical wire-level types.
pub use crate::openapi::AgentStatus;
pub use crate::openapi::MonitoringResult;
pub use crate::openapi::{
    CheckType, Endpoint, HttpGetCheck, HttpGetCheckType, HttpGetResult, PingCheck, PingCheckType,
    PingResult, PluginCheck, PluginCheckType, PluginResult, TcpConnectCheck, TcpConnectCheckType,
    TcpConnectResult, TracerouteCheck, TracerouteCheckType, TracerouteHop, TracerouteResult,
    UdpConnectCheck, UdpConnectCheckType, UdpConnectResult,
};

impl MonitoringResult {
    /// Helper method to determine if the check was successful
    pub fn is_successful(&self) -> bool {
        match &self.check_type {
            CheckType::PingCheck(c) => c.result.successes.unwrap_or(0) > 0,
            CheckType::TracerouteCheck(c) => c.result.target_reached.unwrap_or(false),
            CheckType::TcpConnectCheck(c) => c.result.connected.unwrap_or(false),
            CheckType::UdpConnectCheck(c) => c.result.probe_successful.unwrap_or(false),
            CheckType::HttpGetCheck(c) => c.result.success.unwrap_or(false),
            CheckType::PluginCheck(c) => c.result.success.unwrap_or(false),
        }
    }

    /// Helper method to get the primary response time
    pub fn response_time_ms(&self) -> Option<f64> {
        match &self.check_type {
            CheckType::PingCheck(c) => c.result.avg_response_time_ms,
            CheckType::TracerouteCheck(c) => c.result.total_time_ms,
            CheckType::TcpConnectCheck(c) => c.result.connect_time_ms,
            CheckType::UdpConnectCheck(c) => c.result.response_time_ms,
            CheckType::HttpGetCheck(c) => c.result.response_time_ms,
            CheckType::PluginCheck(c) => c.result.response_time_ms,
        }
    }

    /// Helper method to get the primary error message
    pub fn error_message(&self) -> Option<String> {
        match &self.check_type {
            CheckType::PingCheck(c) => {
                let errors = c.result.errors.as_deref().unwrap_or(&[]);
                if errors.is_empty() {
                    None
                } else {
                    Some(errors.join("; "))
                }
            }
            CheckType::TracerouteCheck(c) => {
                let errors = c.result.errors.as_deref().unwrap_or(&[]);
                if errors.is_empty() {
                    None
                } else {
                    Some(errors.join("; "))
                }
            }
            CheckType::TcpConnectCheck(c) => c.result.error.clone(),
            CheckType::UdpConnectCheck(c) => c.result.error.clone(),
            CheckType::HttpGetCheck(c) => c.result.error.clone(),
            CheckType::PluginCheck(c) => c.result.error.clone(),
        }
    }
}

impl Endpoint {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            id: None,
            address: address.into(),
            port: None,
            enabled: true,
            tags: None,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port as i64);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Agent heartbeat status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentHealthStatus {
    #[default]
    Healthy,
    Degraded,
}

/// Agent heartbeat data sent to the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHeartbeat {
    /// Timestamp when the heartbeat was generated
    pub timestamp: DateTime<Utc>,
    /// Current health status of the agent
    #[serde(default)]
    pub status: AgentHealthStatus,
    /// CPU usage percentage (0.0 to 100.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_usage_percent: Option<f32>,
    /// Memory usage percentage (0.0 to 100.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_usage_percent: Option<f32>,
}

impl AgentHeartbeat {
    /// Create a new heartbeat with the current timestamp
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            status: AgentHealthStatus::Healthy,
            cpu_usage_percent: None,
            memory_usage_percent: None,
        }
    }

    /// Create a heartbeat with system metrics
    pub fn with_metrics(cpu_usage: Option<f32>, memory_usage: Option<f32>) -> Self {
        Self {
            timestamp: Utc::now(),
            status: AgentHealthStatus::Healthy,
            cpu_usage_percent: cpu_usage,
            memory_usage_percent: memory_usage,
        }
    }

    /// Set the health status
    pub fn with_status(mut self, status: AgentHealthStatus) -> Self {
        self.status = status;
        self
    }
}

impl Default for AgentHeartbeat {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_endpoint_enabled_by_default() {
        let endpoint = Endpoint::new("example.com");
        assert!(endpoint.enabled, "Endpoint should be enabled by default");
    }

    #[test]
    fn test_endpoint_with_enabled() {
        let endpoint = Endpoint::new("example.com").with_enabled(false);
        assert!(!endpoint.enabled, "Endpoint should be disabled");

        let endpoint = Endpoint::new("example.com").with_enabled(true);
        assert!(endpoint.enabled, "Endpoint should be enabled");
    }

    #[test]
    fn test_endpoint_serialization() {
        let endpoint = Endpoint::new("example.com").with_enabled(false);
        let json = serde_json::to_string(&endpoint).unwrap();
        assert!(
            json.contains(r#""enabled":false"#),
            "Serialized JSON should contain enabled field"
        );
    }

    #[test]
    fn test_endpoint_with_port() {
        let endpoint = Endpoint::new("example.com").with_port(8080);
        assert_eq!(endpoint.port, Some(8080i64), "Port should be set as i64");
    }

    #[test]
    fn test_endpoint_with_tags() {
        let endpoint = Endpoint::new("example.com").with_tags(vec!["db".to_string()]);
        assert_eq!(endpoint.tags, Some(vec!["db".to_string()]));
    }

    #[test]
    fn test_agent_status_new_with_uuid() {
        let agent_id = Uuid::new_v4();
        let status = AgentStatus::new(agent_id);
        assert_eq!(status.agent_id, agent_id, "Agent ID should match");
        assert!(!status.is_running, "Agent should not be running by default");
        assert_eq!(status.checks_performed, 0, "Checks performed should be 0");
        assert_eq!(
            status.agent_version,
            env!("CARGO_PKG_VERSION"),
            "agent_version should match package version"
        );
        assert_eq!(
            status.config_version, 0,
            "config_version should default to 0"
        );
    }

    #[test]
    fn test_agent_status_serialization() {
        let agent_id = Uuid::new_v4();
        let status = AgentStatus::new(agent_id);
        let json = serde_json::to_string(&status).unwrap();
        assert!(
            json.contains(&agent_id.to_string()),
            "Serialized JSON should contain agent_id as UUID string"
        );
        assert!(
            json.contains("\"agent_version\""),
            "Serialized JSON should contain agent_version field"
        );
        assert!(
            json.contains("\"config_version\""),
            "Serialized JSON should contain config_version field"
        );
    }

    #[test]
    fn test_agent_status_deserialization() {
        let agent_id = Uuid::new_v4();
        let json = format!(
            r#"{{"agent_id":"{}","agent_version":"0.1.0","config_version":0,"is_running":false,"started_at":"1970-01-01T00:00:00Z","stopped_at":null,"checks_performed":0,"checks_successful":0,"checks_failed":0,"last_report_at":"1970-01-01T00:00:00Z","failed_report_count":0,"server_connected":false,"cache_stats":{{"len":0,"capacity":0}}}}"#,
            agent_id
        );
        let status: AgentStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(
            status.agent_id, agent_id,
            "Deserialized agent_id should match"
        );
        assert_eq!(status.agent_version, "0.1.0", "agent_version should match");
        assert_eq!(status.config_version, 0, "config_version should be 0");
    }
}
