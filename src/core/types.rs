//! Common types used throughout the agent

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

// Re-export AgentStatus from the OpenAPI generated models so the rest of the
// codebase gets the canonical wire-compatible type with no local duplicate.
pub use crate::openapi::AgentStatus;

/// Result of a monitoring check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringResult {
    /// Unique ID for this result
    pub id: Uuid,
    /// ID of the agent that performed the check
    pub agent_id: Uuid,
    /// Target endpoint that was checked
    pub target: Endpoint,
    /// Type of check performed with detailed results
    pub check_type: CheckType,
    /// Timestamp when the check was performed
    pub timestamp: DateTime<Utc>,
}

impl MonitoringResult {
    /// Helper method to determine if the check was successful
    pub fn is_successful(&self) -> bool {
        match &self.check_type {
            CheckType::Ping(result) => result.successes > 0,
            CheckType::Traceroute(result) => result.target_reached,
            CheckType::TcpConnect(result) => result.connected,
            CheckType::UdpConnect(result) => result.probe_successful,
            CheckType::HttpGet(result) => result.success,
            CheckType::Plugin(result) => result.success,
        }
    }

    /// Helper method to get the primary response time
    pub fn response_time_ms(&self) -> Option<f64> {
        match &self.check_type {
            CheckType::Ping(result) => result.avg_response_time_ms,
            CheckType::Traceroute(result) => result.total_time_ms,
            CheckType::TcpConnect(result) => result.connect_time_ms,
            CheckType::UdpConnect(result) => result.response_time_ms,
            CheckType::HttpGet(result) => result.response_time_ms,
            CheckType::Plugin(result) => result.response_time_ms,
        }
    }

    /// Helper method to get the primary error message
    pub fn error_message(&self) -> Option<String> {
        match &self.check_type {
            CheckType::Ping(result) => {
                if result.errors.is_empty() {
                    None
                } else {
                    Some(result.errors.join("; "))
                }
            }
            CheckType::Traceroute(result) => {
                if result.errors.is_empty() {
                    None
                } else {
                    Some(result.errors.join("; "))
                }
            }
            CheckType::TcpConnect(result) => result.error.clone(),
            CheckType::UdpConnect(result) => result.error.clone(),
            CheckType::HttpGet(result) => result.error.clone(),
            CheckType::Plugin(result) => result.error.clone(),
        }
    }
}

/// Type of monitoring check with detailed results
// NOTE: We do not derive `Eq` here because the enum variants contain types with floating point fields (e.g., `f64` for response times).
// Rust's `Eq` trait cannot be implemented for types containing floats due to the possibility of NaN values, which do not satisfy equality.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CheckType {
    Ping(PingResult),
    Traceroute(TracerouteResult),
    TcpConnect(TcpConnectResult),
    UdpConnect(UdpConnectResult),
    HttpGet(HttpGetResult),
    Plugin(PluginResult),
}

/// Result of a ping check
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PingResult {
    /// Resolved IP address
    pub resolved_ip: Option<String>,
    /// Number of successfully received replies
    pub successes: u32,
    /// Number of timeouts or other ICMP related errors
    pub failures: u32,
    /// Latency for each successful check in milliseconds
    pub success_latencies: Vec<f64>,
    /// Average response time in milliseconds
    pub avg_response_time_ms: Option<f64>,
    /// ICMP error messages
    pub errors: Vec<String>,
}

/// Result of a traceroute check
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TracerouteResult {
    /// List of hops in the traceroute
    pub hops: Vec<TracerouteHop>,
    /// Whether the target was reached
    pub target_reached: bool,
    /// Total time for the traceroute
    pub total_time_ms: Option<f64>,
    /// Any errors encountered
    pub errors: Vec<String>,
}

/// Traceroute hop information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TracerouteHop {
    /// Hop number (TTL)
    pub hop: u8,
    /// IP address of the hop
    pub address: Option<IpAddr>,
    /// Response time in milliseconds
    pub response_time_ms: Option<f64>,
    /// Hostname if resolved
    pub hostname: Option<String>,
}

/// Result of a TCP connection check
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TcpConnectResult {
    /// Whether the connection was successful
    pub connected: bool,
    /// Time to establish connection in milliseconds
    pub connect_time_ms: Option<f64>,
    /// Error message if connection failed
    pub error: Option<String>,
    /// Resolved IP address
    pub resolved_ip: Option<String>,
}

/// Result of a UDP connection check
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UdpConnectResult {
    /// Whether the UDP probe was successful
    pub probe_successful: bool,
    /// Response time in milliseconds
    pub response_time_ms: Option<f64>,
    /// Error message if probe failed
    pub error: Option<String>,
    /// Resolved IP address
    pub resolved_ip: Option<String>,
}

/// Result of an HTTP GET check
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HttpGetResult {
    /// HTTP status code
    pub status_code: Option<u16>,
    /// Response time in milliseconds
    pub response_time_ms: Option<f64>,
    /// Size of response body in bytes
    pub response_size_bytes: Option<usize>,
    /// Error message if request failed
    pub error: Option<String>,
    /// Whether the request was successful
    pub success: bool,
}

/// Result of a plugin check
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginResult {
    /// Name of the plugin
    pub plugin_name: String,
    /// Version of the plugin
    pub plugin_version: String,
    /// Whether the check was successful
    pub success: bool,
    /// Response time in milliseconds
    pub response_time_ms: Option<f64>,
    /// Error message if check failed
    pub error: Option<String>,
    /// Plugin-specific data
    pub data: std::collections::HashMap<String, String>,
}

/// Endpoint to monitor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    /// Target address (hostname or IP)
    pub address: String,
    /// Optional port number
    pub port: Option<u16>,
    /// Tags for organizing endpoints
    pub tags: Vec<String>,
    /// Whether this endpoint is enabled for monitoring
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// Default value for enabled field
fn default_enabled() -> bool {
    true
}

impl Endpoint {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            port: None,
            tags: Vec::new(),
            enabled: true,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
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
    fn test_endpoint_deserialization_with_enabled() {
        let json = r#"{"address": "example.com", "port": null, "tags": [], "enabled": false}"#;
        let endpoint: Endpoint = serde_json::from_str(json).unwrap();
        assert!(
            !endpoint.enabled,
            "Deserialized endpoint should be disabled"
        );
    }

    #[test]
    fn test_endpoint_deserialization_without_enabled() {
        // When enabled field is missing, it should default to true
        let json = r#"{"address": "example.com", "port": null, "tags": []}"#;
        let endpoint: Endpoint = serde_json::from_str(json).unwrap();
        assert!(
            endpoint.enabled,
            "Endpoint without enabled field should default to true"
        );
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
        assert_eq!(status.config_version, 0, "config_version should default to 0");
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
            r#"{{"agent_id":"{}","agent_version":"0.1.0","config_version":0,"is_running":false,"started_at":"1970-01-01T00:00:00Z","stopped_at":null,"checks_performed":0,"checks_successful":0,"checks_failed":0,"last_report_at":"1970-01-01T00:00:00Z","failed_report_count":0,"server_connected":false,"cached_reports":0}}"#,
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
