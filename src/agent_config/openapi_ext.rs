//! Default implementations and typed accessor methods for generated OpenAPI config types.

use std::time::Duration;
use uuid::Uuid;

use crate::openapi::{
    AgentConfig, ClaimConfig, MonitoringConfig, SelfUpgradeConfig, ServerConfig, StorageConfig,
};

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            // 0 means unregistered; set to 1+ after pulling config from server
            version: 0,
            // nil UUID means unregistered; set after registration
            agent_id: Uuid::nil(),
            agent_name: String::from("Unnamed Agent"),
            tags: Vec::new(),
            monitoring: MonitoringConfig::default(),
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            self_upgrade: SelfUpgradeConfig::default(),
            endpoints: Vec::new(),
        }
    }
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
        Duration::from_secs(self.interval_secs as u64)
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs as u64)
    }
}

impl Default for ClaimConfig {
    fn default() -> Self {
        Self {
            max_registration_retries: 5,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            url: String::from("https://api.smotra.net/v1"),
            api_key: None,
            heartbeat_interval_secs: 300,
            verify_tls: true,
            timeout_secs: 5,
            retry_attempts: 3,
            claiming: ClaimConfig::default(),
        }
    }
}

impl ServerConfig {
    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_secs(self.heartbeat_interval_secs as u64)
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs as u64)
    }

    pub fn is_configured(&self) -> bool {
        !self.url.is_empty() && self.api_key.as_deref().is_some_and(|k| !k.is_empty())
    }

    pub fn is_claim_required(&self) -> bool {
        self.api_key.as_deref().is_none_or(|k| k.is_empty())
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            cache_dir: String::from("./cache"),
            max_cached_results: 10000,
            max_cache_age_secs: 86400,
            cache_enabled: true,
            cache_batch_size: 100,
            cache_report_interval_secs: 60,
        }
    }
}

impl StorageConfig {
    pub fn max_cache_age(&self) -> Duration {
        Duration::from_secs(self.max_cache_age_secs as u64)
    }

    pub fn cache_report_interval(&self) -> Duration {
        Duration::from_secs(self.cache_report_interval_secs as u64)
    }
}

impl Default for SelfUpgradeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            github_repo_url: String::from("https://github.com/smotra-monitoring/agent"),
            check_interval_secs: 3600,
        }
    }
}
