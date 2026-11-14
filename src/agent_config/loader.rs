//! Configuration loading, saving, and validation

use tracing::info;
use uuid::Uuid;

use super::merge_patch::layer_over_defaults;
use crate::claim::AgentCredentials;
use crate::error::{Error, Result};
use crate::openapi::AgentConfig;
use std::fs;
use std::path::Path;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;

/// Lower bound that keeps a server-supplied config from flooding the server with heartbeats.
const MIN_HEARTBEAT_INTERVAL_SECS: i64 = 60;

/// Create a new owner-only file at `path`, write `content` and fsync it.
async fn write_synced_file(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let mut options = async_fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);

    let mut file = options.open(path).await?;
    file.write_all(content).await?;
    file.flush().await?;
    file.sync_all().await
}

impl AgentConfig {
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

    /// Load configuration from a TOML file without validating it.
    /// Keys missing from the file fall back to `AgentConfig::default()`.
    ///
    /// Only for flows that need a not-yet-valid config (e.g. before claiming); otherwise use
    /// [`Self::load_and_validate_config`].
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;

        let local: serde_json::Value = toml::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse toml-config: {}", e)))?;

        serde_json::from_value(layer_over_defaults(&local)?).map_err(|e| {
            Error::Config(format!(
                "Failed to merge 'Default config' with toml-config: {}",
                e
            ))
        })
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

        if self.server.timeout_secs == 0 {
            return Err(Error::Config(
                "server timeout must be greater than 0".to_string(),
            ));
        }

        if self.server.url.is_empty() {
            return Err(Error::Config("server URL cannot be empty".to_string()));
        }

        if self.server.heartbeat_interval_secs < MIN_HEARTBEAT_INTERVAL_SECS {
            return Err(Error::Config(format!(
                "server heartbeat interval must be at least {} seconds",
                MIN_HEARTBEAT_INTERVAL_SECS
            )));
        }

        if self.server.api_key.is_some() && self.server.api_key.as_ref().unwrap().is_empty() {
            return Err(Error::ConfigApiKey(
                "server API key cannot be empty".to_string(),
            ));
        }

        if self.self_upgrade.github_repo_url.is_empty() {
            return Err(Error::Config(
                "self_upgrade.github_repo_url cannot be empty".to_string(),
            ));
        }

        if self.self_upgrade.check_interval_secs <= 0 {
            return Err(Error::Config(
                "self_upgrade.check_interval_secs must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }

    /// Save configuration to a TOML file atomically with secure permissions
    ///
    /// The config is written to a temporary file next to `path` (mode 0600 on Unix, to protect
    /// API keys), synced to disk, then renamed over `path`. On any failure the existing file
    /// is left untouched, so readers see either the old or the new config, never a partial one.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where to save the configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Serialization to TOML fails
    /// * The temporary file cannot be created, written or synced
    /// * The temporary file cannot be renamed over `path`
    pub async fn save_to_file_secure(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;

        // Replace the real file rather than a symlink pointing at it.
        let target = async_fs::canonicalize(path.as_ref())
            .await
            .unwrap_or_else(|_| path.as_ref().to_path_buf());
        let file_name = target
            .file_name()
            .ok_or_else(|| Error::Config("Config path has no file name".to_string()))?;
        let dir = match target.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };

        // Same directory as the target so the rename stays on one filesystem.
        let tmp_path = dir.join(format!(
            ".{}.{}.tmp",
            file_name.to_string_lossy(),
            Uuid::now_v7()
        ));

        if let Err(e) = write_synced_file(&tmp_path, content.as_bytes()).await {
            let _ = async_fs::remove_file(&tmp_path).await;
            return Err(Error::Config(format!(
                "Failed to write temporary config file: {}",
                e
            )));
        }

        if let Err(e) = async_fs::rename(&tmp_path, &target).await {
            let _ = async_fs::remove_file(&tmp_path).await;
            return Err(Error::Config(format!(
                "Failed to replace config file: {}",
                e
            )));
        }

        // Persist the rename itself across power loss.
        #[cfg(unix)]
        {
            let dir_handle = async_fs::File::open(dir)
                .await
                .map_err(|e| Error::Config(format!("Failed to open config directory: {}", e)))?;
            dir_handle
                .sync_all()
                .await
                .map_err(|e| Error::Config(format!("Failed to sync config directory: {}", e)))?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim::AgentCredentials;
    use crate::openapi::{AgentConfig, ServerConfig};
    use tempfile::NamedTempFile;

    #[test]
    fn test_apply_claim_result() {
        let mut config = AgentConfig::default();
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
        let mut config = AgentConfig {
            agent_id: old_agent_id,
            server: ServerConfig {
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
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let agent_id = Uuid::now_v7();
        let config = AgentConfig {
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
    async fn test_save_overwrites_atomically_without_leftovers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut config = AgentConfig {
            agent_name: "First".to_string(),
            ..Default::default()
        };
        config.save_to_file_secure(&path).await.unwrap();

        config.agent_name = "Second".to_string();
        config.save_to_file_secure(&path).await.unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("Second") && !content.contains("First"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }

        let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1, "temporary file must not be left behind");
    }

    #[tokio::test]
    async fn test_save_failure_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        // A directory cannot be replaced by a file, so the final rename fails.
        let target = dir.path().join("config.toml");
        std::fs::create_dir(&target).unwrap();

        let result = AgentConfig::default().save_to_file_secure(&target).await;
        assert!(result.is_err());

        let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1, "temporary file must be cleaned up");
    }

    #[tokio::test]
    async fn test_apply_claim_result_and_save_secure() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Start with default config
        let mut config = AgentConfig {
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
        let loaded_config = AgentConfig::from_file(path).unwrap();
        assert_eq!(loaded_config.agent_id, agent_id);
        assert_eq!(
            loaded_config.server.api_key,
            Some("sk_integration_test".to_string())
        );
        assert_eq!(loaded_config.agent_name, "Integration Test Agent");
    }

    #[test]
    fn test_from_file_fills_missing_keys_with_defaults() {
        let file = NamedTempFile::new().unwrap();
        std::fs::write(
            file.path(),
            "agent_name = \"partial\"\n[monitoring]\ninterval_secs = 5\n",
        )
        .unwrap();

        let config = AgentConfig::from_file(file.path()).unwrap();

        assert_eq!(config.agent_name, "partial");
        assert_eq!(config.monitoring.interval_secs, 5);
        assert_eq!(config.monitoring.timeout_secs, 1);
        assert_eq!(config.server.url, ServerConfig::default().url);
    }

    #[test]
    fn test_validate_fails_when_self_upgrade_url_is_empty() {
        let mut config = AgentConfig {
            agent_id: Uuid::now_v7(),
            ..Default::default()
        };
        config.self_upgrade.github_repo_url = String::new();

        let result = config.validate();
        assert!(
            result.is_err(),
            "empty self_upgrade.github_repo_url should fail validation"
        );
    }

    #[test]
    fn test_validate_fails_when_self_upgrade_interval_is_zero() {
        let mut config = AgentConfig {
            agent_id: Uuid::now_v7(),
            ..Default::default()
        };
        config.self_upgrade.check_interval_secs = 0;

        let result = config.validate();
        assert!(
            result.is_err(),
            "zero self_upgrade.check_interval_secs should fail validation"
        );
    }

    /// Heartbeat intervals below the 60 s floor are rejected; 60 s itself is accepted.
    #[test]
    fn test_validate_heartbeat_interval_minimum() {
        let mut config = AgentConfig {
            agent_id: Uuid::now_v7(),
            ..Default::default()
        };

        config.server.heartbeat_interval_secs = 59;
        assert!(
            config.validate().is_err(),
            "59 s heartbeat interval should fail validation"
        );

        config.server.heartbeat_interval_secs = 0;
        assert!(
            config.validate().is_err(),
            "zero heartbeat interval should fail validation"
        );

        config.server.heartbeat_interval_secs = 60;
        assert!(
            config.validate().is_ok(),
            "60 s heartbeat interval should pass validation"
        );
    }
}
