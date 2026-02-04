//! Configuration loading, saving, and validation

use super::Config;
use crate::error::{Error, Result};
use std::fs;
use std::path::Path;

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse config: {}", e)))
    }

    /// Save configuration to a TOML file
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(path.as_ref(), content)
            .map_err(|e| Error::Config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.agent_id.is_empty() {
            return Err(Error::Config("agent_id cannot be empty".to_string()));
        }

        if self.monitoring.interval_secs == 0 {
            return Err(Error::Config(
                "monitoring interval must be greater than 0".to_string(),
            ));
        }

        if self.server.report_interval_secs == 0 {
            return Err(Error::Config(
                "server report_interval must be greater than 0".to_string(),
            ));
        }

        if self.server.timeout_secs == 0 {
            return Err(Error::Config(
                "server timeout must be greater than 0".to_string(),
            ));
        }

        if self.server.report_interval_secs < self.server.timeout_secs * 2 {
            return Err(Error::Config(
                "server report_interval should be at least two times greater than the monitoring interval"
                    .to_string(),
            ));
        }

        Ok(())
    }
}
