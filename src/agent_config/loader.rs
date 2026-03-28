//! Configuration loading, saving, and validation

use tracing::info;
use uuid::Uuid;

use super::Config;
use crate::claim::AgentCredentials;
use crate::error::{Error, Result};
use std::fs;
use std::path::Path;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse config: {}", e)))
    }

    /// Save configuration to a TOML file asynchronously with secure permissions
    ///
    /// This method writes the configuration and sets file permissions to 0600
    /// on Unix systems (owner read/write only) to protect sensitive data like API keys.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where to save the configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Serialization to TOML fails
    /// * File cannot be created or written
    /// * Permissions cannot be set (Unix only)
    pub async fn save_to_file_secure(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;

        // Write to file
        let mut file = async_fs::File::create(path)
            .await
            .map_err(|e| Error::Config(format!("Failed to create config file: {}", e)))?;

        file.write_all(content.as_bytes())
            .await
            .map_err(|e| Error::Config(format!("Failed to write config file: {}", e)))?;

        file.flush()
            .await
            .map_err(|e| Error::Config(format!("Failed to flush config file: {}", e)))?;

        // Set secure permissions on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut perms = file
                .metadata()
                .await
                .map_err(|e| Error::Config(format!("Failed to read file metadata: {}", e)))?
                .permissions();
            perms.set_mode(0o600); // Owner read/write only
            async_fs::set_permissions(path, perms)
                .await
                .map_err(|e| Error::Config(format!("Failed to set file permissions: {}", e)))?;
        }

        Ok(())
    }

    /// Apply claim result to configuration
    ///
    /// Updates the configuration with the agent ID and API key obtained
    /// from the claiming workflow. This method should be called before
    /// saving the configuration to persist the claim result.
    ///
    /// # Arguments
    ///
    /// * `claim_result` - Result from the claiming workflow containing API key and agent ID
    pub fn apply_claim_result(&mut self, claim_result: AgentCredentials) {
        self.agent_id = claim_result.agent_id;
        self.server.api_key = Some(claim_result.api_key);
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.agent_id == Uuid::nil() {
            return Err(Error::Config("agent_id cannot be nil UUID".to_string()));
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

        if self.server.url.is_empty() {
            return Err(Error::Config("server URL cannot be empty".to_string()));
        }

        if self.server.api_key.is_some() && self.server.api_key.as_ref().unwrap().is_empty() {
            return Err(Error::ConfigApiKey(
                "server API key cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Load and validate configuration from file
    ///
    /// Convenience function that loads a config file and validates it in one step.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * File cannot be read
    /// * TOML parsing fails
    /// * Configuration validation fails
    pub fn load_and_validate_config(path: impl AsRef<Path>) -> Result<Self> {
        info!("Loading config from: {:?}", path.as_ref());
        let config = Self::from_file(path)?;
        config.validate()?;
        info!("Config loaded and validated successfully");
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim::AgentCredentials;
    use tempfile::NamedTempFile;

    #[test]
    fn test_apply_claim_result() {
        let mut config = Config::default();
        assert_eq!(config.agent_id, Uuid::nil());
        assert_eq!(config.server.api_key, None);

        let new_agent_id = Uuid::now_v7();
        let claim_result = AgentCredentials {
            api_key: "sk_test_123456".to_string(),
            agent_id: new_agent_id,
        };

        config.apply_claim_result(claim_result);

        assert_eq!(config.agent_id, new_agent_id);
        assert_eq!(config.server.api_key, Some("sk_test_123456".to_string()));
    }

    #[test]
    fn test_apply_claim_result_overwrites_existing() {
        let old_agent_id = Uuid::now_v7();
        let mut config = Config {
            agent_id: old_agent_id,
            server: crate::ServerConfig {
                api_key: Some("old_key".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let new_agent_id = Uuid::now_v7();
        let claim_result = AgentCredentials {
            api_key: "new_key".to_string(),
            agent_id: new_agent_id,
        };

        config.apply_claim_result(claim_result);

        assert_eq!(config.agent_id, new_agent_id);
        assert_eq!(config.server.api_key, Some("new_key".to_string()));
    }

    #[tokio::test]
    async fn test_save_to_file_secure() {
        use crate::agent_config::server_config::ServerConfig;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let agent_id = Uuid::now_v7();
        let config = Config {
            agent_id,
            agent_name: "Test Agent".to_string(),
            server: ServerConfig {
                api_key: Some("sk_test_secure".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = config.save_to_file_secure(path).await;
        assert!(result.is_ok(), "Failed to save config: {:?}", result);

        // Verify file contents
        let content = async_fs::read_to_string(path).await.unwrap();
        assert!(content.contains("sk_test_secure"));
        assert!(content.contains(&agent_id.to_string()));
        assert!(content.contains("Test Agent"));

        // Verify file permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(path).unwrap();
            let permissions = metadata.permissions();
            assert_eq!(
                permissions.mode() & 0o777,
                0o600,
                "File permissions should be 0600"
            );
        }
    }

    #[tokio::test]
    async fn test_apply_claim_result_and_save_secure() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Start with default config
        let mut config = Config {
            agent_name: "Integration Test Agent".to_string(),
            ..Default::default()
        };

        // Apply claim result
        let agent_id = Uuid::now_v7();
        let claim_result = AgentCredentials {
            api_key: "sk_integration_test".to_string(),
            agent_id,
        };
        config.apply_claim_result(claim_result);

        // Save securely
        config.save_to_file_secure(path).await.unwrap();

        // Reload and verify
        let loaded_config = Config::from_file(path).unwrap();
        assert_eq!(loaded_config.agent_id, agent_id);
        assert_eq!(
            loaded_config.server.api_key,
            Some("sk_integration_test".to_string())
        );
        assert_eq!(loaded_config.agent_name, "Integration Test Agent");
    }
}
