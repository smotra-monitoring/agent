//! Server configuration types

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Server connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server URL
    pub url: String,

    /// API key for authentication
    pub api_key: Option<String>,

    /// Report interval in seconds
    pub report_interval_secs: u64,

    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,

    /// Enable TLS verification
    pub verify_tls: bool,

    /// Connection timeout in seconds
    pub timeout_secs: u64,

    /// Retry attempts on failure
    pub retry_attempts: u32,

    /// Claiming configuration
    pub claiming: ClaimConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            url: "https://api.smotra.net".to_string(),
            api_key: None,
            report_interval_secs: 300,
            heartbeat_interval_secs: 300,
            verify_tls: true,
            timeout_secs: 5,
            retry_attempts: 3,
            claiming: ClaimConfig::default(),
        }
    }
}

impl ServerConfig {
    pub fn report_interval(&self) -> Duration {
        Duration::from_secs(self.report_interval_secs)
    }

    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_secs(self.heartbeat_interval_secs)
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }

    pub fn is_configured(&self) -> bool {
        !self.url.is_empty()
            && (self.api_key.is_some() && !self.api_key.as_ref().unwrap().is_empty())
    }
}

/// Claiming workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimConfig {
    /// Polling interval while waiting for claim (in seconds)
    pub poll_interval_secs: u64,

    /// Maximum registration retry attempts
    pub max_registration_retries: u32,
}

impl Default for ClaimConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 30,
            max_registration_retries: 5,
        }
    }
}

impl ClaimConfig {
    pub fn poll_interval(&self) -> Duration {
        Duration::from_secs(self.poll_interval_secs)
    }
}
