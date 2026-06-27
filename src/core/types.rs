//! Common types used throughout the agent

use uuid::Uuid;

// Re-export from generated OpenAPI types — these are the canonical wire-level types.
pub use crate::openapi::AgentStatus;
pub use crate::openapi::MonitoringResult;
pub use crate::openapi::{
    AgentHealthStatus, AgentHeartbeat, AgentMetrics, CheckType, Endpoint, ErrorDetails,
    HttpGetCheck, HttpGetCheckType, HttpGetResult, PingCheck, PingCheckType, PingResult,
    PluginCheck, PluginCheckType, PluginResult, TcpConnectCheck, TcpConnectCheckType,
    TcpConnectResult, TracerouteCheck, TracerouteCheckType, TracerouteHop, TracerouteResult,
    UdpConnectCheck, UdpConnectCheckType, UdpConnectResult,
};

impl MonitoringResult {
    /// Helper method to determine if the check was successful
    pub fn is_successful(&self) -> bool {
        match &self.check_type {
            CheckType::PingCheck(c) => c.result.successes > 0,
            CheckType::TracerouteCheck(c) => c.result.target_reached,
            CheckType::TcpConnectCheck(c) => c.result.connected,
            CheckType::UdpConnectCheck(c) => c.result.probe_successful,
            CheckType::HttpGetCheck(c) => c.result.success,
            CheckType::PluginCheck(c) => c.result.success,
        }
    }

    /// Helper method to get the primary response time
    pub fn response_time_ms(&self) -> Option<f64> {
        match &self.check_type {
            CheckType::PingCheck(c) => {
                let latencies = &c.result.success_latencies;
                if latencies.is_empty() {
                    None
                } else {
                    Some(latencies.iter().sum::<f64>() / latencies.len() as f64)
                }
            }
            CheckType::TracerouteCheck(c) => c.result.hops.last().and_then(|h| {
                h.success_latencies.as_deref().and_then(|latencies| {
                    if latencies.is_empty() {
                        None
                    } else {
                        let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
                        Some(avg)
                    }
                })
            }),
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
                let errors = c
                    .result
                    .error_details
                    .as_ref()
                    .and_then(|ed| ed.errors.as_deref())
                    .unwrap_or(&[]);
                if errors.is_empty() {
                    None
                } else {
                    Some(errors.join("; "))
                }
            }
            CheckType::TracerouteCheck(c) => {
                let errors = c
                    .result
                    .error_details
                    .as_ref()
                    .and_then(|ed| ed.errors.as_deref())
                    .unwrap_or(&[]);
                if errors.is_empty() {
                    None
                } else {
                    Some(errors.join("; "))
                }
            }
            CheckType::TcpConnectCheck(c) => {
                let errors = c
                    .result
                    .error_details
                    .as_ref()
                    .and_then(|ed| ed.errors.as_deref())
                    .unwrap_or(&[]);
                if errors.is_empty() {
                    None
                } else {
                    Some(errors.join("; "))
                }
            }
            CheckType::UdpConnectCheck(c) => {
                let errors = c
                    .result
                    .error_details
                    .as_ref()
                    .and_then(|ed| ed.errors.as_deref())
                    .unwrap_or(&[]);
                if errors.is_empty() {
                    None
                } else {
                    Some(errors.join("; "))
                }
            }
            CheckType::HttpGetCheck(c) => {
                let errors = c
                    .result
                    .error_details
                    .as_ref()
                    .and_then(|ed| ed.errors.as_deref())
                    .unwrap_or(&[]);
                if errors.is_empty() {
                    None
                } else {
                    Some(errors.join("; "))
                }
            }
            CheckType::PluginCheck(c) => {
                let errors = c
                    .result
                    .error_details
                    .as_ref()
                    .and_then(|ed| ed.errors.as_deref())
                    .unwrap_or(&[]);
                if errors.is_empty() {
                    None
                } else {
                    Some(errors.join("; "))
                }
            }
        }
    }
}

impl Endpoint {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            id: Uuid::now_v7(),
            address: address.into(),
            port: None,
            enabled: true,
            tags: Vec::new(),
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port as i64);
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_endpoint_enabled_by_default() {
        let endpoint = Endpoint::new("example.com");
        assert!(endpoint.enabled, "Endpoint should be enabled by default");
        assert_ne!(
            endpoint.id,
            Uuid::nil(),
            "Endpoint::new() should generate a non-nil UUID"
        );
    }

    #[test]
    fn test_endpoint_new_generates_unique_ids() {
        let a = Endpoint::new("example.com");
        let b = Endpoint::new("example.com");
        assert_ne!(
            a.id, b.id,
            "Each Endpoint::new() call must produce a distinct UUIDv7"
        );
    }

    #[test]
    fn test_endpoint_missing_id_fails_deserialization() {
        // Endpoints without an id must fail at deserialization — there is no serde default.
        let toml_input = r#"
[[endpoints]]
address = "8.8.8.8"
enabled = true
tags = []
"#;
        // The failure occurs when deserializing into a typed struct that requires `id`.
        #[derive(serde::Deserialize)]
        struct Wrapper {
            #[allow(dead_code)]
            endpoints: Vec<crate::openapi::Endpoint>,
        }
        let typed: Result<Wrapper, _> = toml::from_str(toml_input);
        assert!(
            typed.is_err(),
            "Deserializing an endpoint without 'id' must fail"
        );
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
        assert!(
            json.contains("\"id\""),
            "Serialized JSON should contain id field"
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
        assert_eq!(
            endpoint.tags,
            vec!["db".to_string()],
            "Tags should be set correctly"
        );
    }

    #[test]
    fn test_agent_status_new_with_uuid() {
        let agent_id = Uuid::now_v7();
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
        let agent_id = Uuid::now_v7();
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
        let agent_id = Uuid::now_v7();
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
