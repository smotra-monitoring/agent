//! Common types used throughout the agent

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

/// Result of a monitoring check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringResult {
    /// Unique ID for this result
    pub id: Uuid,
    /// ID of the agent that performed the check
    pub agent_id: String,
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

/// Current status of the agent
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentStatus {
    /// Unique identifier for this agent
    pub agent_id: String,
    /// Whether the agent is currently running
    pub is_running: bool,
    /// When the agent was started
    pub started_at: Option<DateTime<Utc>>,
    /// When the agent was shutdown
    pub stopped_at: Option<DateTime<Utc>>,
    /// Number of checks performed
    pub checks_performed: u64,
    /// Number of successful checks
    pub checks_successful: u64,
    /// Number of failed checks
    pub checks_failed: u64,
    /// Last time data was sent to server
    pub last_report_at: Option<DateTime<Utc>>,
    /// Number of failed report attempts
    pub failed_report_count: u64,
    /// Whether the agent is connected to the server
    pub server_connected: bool,
    /// Number of cached results waiting to be sent
    pub cached_results: usize,
}

impl AgentStatus {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            ..Default::default()
        }
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
}
