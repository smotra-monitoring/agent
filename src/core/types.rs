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
    /// Type of check performed
    pub check_type: CheckType,
    /// Whether the check was successful
    pub success: bool,
    /// Response time in milliseconds
    pub response_time_ms: Option<f64>,
    /// Error message if check failed
    pub error: Option<String>,
    /// Timestamp when the check was performed
    pub timestamp: DateTime<Utc>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Type of monitoring check
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CheckType {
    Ping,
    Traceroute,
    TcpConnect,
    UdpConnect,
    HttpGet,
    Plugin(String),
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
}

impl Endpoint {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            port: None,
            tags: Vec::new(),
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
}

/// Current status of the agent
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentStatus {
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
    /// Whether the agent is connected to the server
    pub server_connected: bool,
    /// Number of cached results waiting to be sent
    pub cached_results: usize,
}

/// Traceroute hop information
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Complete traceroute result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteResult {
    /// Target address
    pub target: String,
    /// List of hops
    pub hops: Vec<TracerouteHop>,
    /// Whether the target was reached
    pub reached: bool,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}
