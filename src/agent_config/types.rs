//! Configuration types

use super::server_config::ServerConfig;
use crate::core::Endpoint;
use crate::openapi;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// Main configuration for the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Configuration version (used for syncing with server)
    /// 0 means unregistered, will be set to 1+ after pulling registration from server
    pub version: u32,

    /// Unique identifier for this agent
    pub agent_id: Uuid,

    /// Human-readable agent name
    pub agent_name: String,

    /// Tags for this agent (used for mesh organization)
    pub tags: Vec<String>,

    /// Monitoring configuration
    pub monitoring: MonitoringConfig,

    /// Server configuration
    pub server: ServerConfig,

    /// Local storage configuration
    pub storage: StorageConfig,

    /// Self-upgrade configuration (OpenAPI-generated type)
    pub update: openapi::UpdateConfig,

    /// Endpoints to monitor
    pub endpoints: Vec<Endpoint>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 0, // 0 means unregistered, will be set to 1+ after pulling registration from server
            agent_id: Uuid::nil(), // nil UUID means unregistered, will be set after registration
            agent_name: String::from("Unnamed Agent"),
            tags: Vec::new(),
            monitoring: MonitoringConfig::default(),
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            update: openapi::UpdateConfig::default(),
            endpoints: Vec::new(),
        }
    }
}

impl Default for openapi::UpdateConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            github_repo_url: "https://github.com/smotra-monitoring/agent".to_string(),
            check_interval_secs: 3600,
        }
    }
}

/// Monitoring-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Interval between checks in seconds
    pub interval_secs: u64,

    /// Timeout for each check in seconds
    pub timeout_secs: u64,

    /// Number of pings to send per check
    pub ping_count: u32,

    /// Maximum number of concurrent checks
    pub max_concurrent: usize,

    /// Enable traceroute on failed pings
    pub traceroute_on_failure: bool,

    /// Maximum TTL for traceroute
    pub traceroute_max_hops: u8,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            interval_secs: 60,
            timeout_secs: 1,
            ping_count: 3,
            max_concurrent: 10,
            traceroute_on_failure: false,
            traceroute_max_hops: 30,
        }
    }
}

impl MonitoringConfig {
    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.interval_secs)
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }
}

/// Local storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Directory for storing cached data
    pub cache_dir: String,

    /// Maximum number of results to cache
    pub max_cached_results: usize,

    /// Maximum age of cached results in seconds
    pub max_cache_age_secs: u64,

    /// Enable in-memory result caching and batch reporting.
    /// When disabled, results are never buffered locally and
    /// the result reporter loop exits immediately.
    pub cache_enabled: bool,

    /// Number of results sent to the server in a single POST request.
    pub cache_batch_size: usize,

    /// How often (in seconds) the result reporter drains the cache
    /// and attempts to send a batch to the server.
    pub cache_report_interval_secs: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            cache_dir: "./cache".to_string(),
            max_cached_results: 10000,
            max_cache_age_secs: 86400, // 24 hours
            cache_enabled: true,
            cache_batch_size: 100,
            cache_report_interval_secs: 60,
        }
    }
}

impl StorageConfig {
    pub fn max_cache_age(&self) -> Duration {
        Duration::from_secs(self.max_cache_age_secs)
    }
}
