//! Secure server configuration persistence
//!
//! This module handles saving and loading server configuration,
//! especially API keys, with proper file permissions.

use crate::error::{Error, Result};
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::info;
use uuid::Uuid;

/// Save server configuration with API key
///
/// Saves the API key to the configuration file with secure permissions (0600 on Unix).
/// This ensures only the file owner can read/write the configuration containing
/// sensitive authentication credentials.
///
/// # Arguments
///
/// * `api_key` - The API key to save
/// * `config_path` - Path to the configuration file
/// * `agent_id` - Agent ID to save in the configuration
///
/// # Errors
///
/// Returns an error if:
/// * The file cannot be created
/// * Permissions cannot be set
/// * Writing to the file fails
pub async fn save_api_key_to_config(
    api_key: &str,
    agent_id: Uuid,
    config_path: &Path,
) -> Result<()> {
    info!("Saving API key to configuration: {}", config_path.display());

    // Read existing config if it exists
    let mut config = if config_path.exists() {
        let content = fs::read_to_string(config_path).await?;
        toml::from_str::<toml::Value>(&content)
            .map_err(|e| Error::Config(format!("Failed to parse TOML config: {}", e)))?
    } else {
        toml::Value::Table(toml::map::Map::new())
    };

    // Update server.api_key
    if let toml::Value::Table(ref mut table) = config {
        let server = table
            .entry("server".to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

        if let toml::Value::Table(ref mut server_table) = server {
            server_table.insert(
                "api_key".to_string(),
                toml::Value::String(api_key.to_string()),
            );
        }

        // Also update agent_id
        table.insert(
            "agent_id".to_string(),
            toml::Value::String(agent_id.to_string()),
        );
    }

    // Serialize to TOML
    let config_str = toml::to_string_pretty(&config)
        .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;

    // Write to file
    let mut file = fs::File::create(config_path).await?;
    file.write_all(config_str.as_bytes()).await?;
    file.flush().await?;

    // Set secure permissions on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = file.metadata().await?.permissions();
        perms.set_mode(0o600); // Owner read/write only
        fs::set_permissions(config_path, perms).await?;

        info!(
            "Set file permissions to 0600 (owner read/write only): {}",
            config_path.display()
        );
    }

    #[cfg(not(unix))]
    {
        // File permission setting not available on this platform
        // On Windows, the file ACLs would need to be set differently
        info!(
            "File permissions not set (check permissions manually on file {})",
            config_path.display()
        );
    }

    info!(
        "API key saved in the config file: {}",
        config_path.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_save_api_key_to_new_config() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let agent_id = Uuid::parse_str("00000000-0000-0000-0000-000000000123").unwrap();
        let result = save_api_key_to_config("sk_test_12345", agent_id, path).await;

        assert!(result.is_ok());

        // Verify file contents
        let content = fs::read_to_string(path).await.unwrap();
        assert!(content.contains("api_key = \"sk_test_12345\""));
        assert!(content.contains(&format!("agent_id = \"{}\"", agent_id)));
    }

    #[tokio::test]
    async fn test_save_api_key_to_existing_config() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let existing_agent_id = Uuid::parse_str("00000000-0000-0000-0000-000000000456").unwrap();
        // Create initial config
        let initial_config = format!(
            r#"
version = 1
agent_id = "{}"
agent_name = "Test Agent"

[server]
url = "https://example.com"
"#,
            existing_agent_id
        );
        fs::write(path, initial_config).await.unwrap();

        // Save API key
        let result = save_api_key_to_config("sk_test_67890", existing_agent_id, path).await;

        assert!(result.is_ok());

        // Verify file contents
        let content = fs::read_to_string(path).await.unwrap();
        assert!(content.contains("api_key = \"sk_test_67890\""));
        assert!(content.contains(&format!("agent_id = \"{}\"", existing_agent_id)));
        assert!(content.contains("url = \"https://example.com\""));
    }

    #[tokio::test]
    async fn test_save_agent_id_to_existing_config() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let old_agent_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let new_agent_id = Uuid::parse_str("00000000-0000-0000-0000-000000000789").unwrap();
        // Create initial config
        let initial_config = format!(
            r#"
version = 1
agent_id = "{}"

[server]
url = "https://example.com"
"#,
            old_agent_id
        );
        fs::write(path, initial_config).await.unwrap();

        // Save API key with new agent_id
        let result = save_api_key_to_config("sk_test_67890", new_agent_id, path).await;

        assert!(result.is_ok());

        // Verify file contents
        let content = fs::read_to_string(path).await.unwrap();
        assert!(content.contains("api_key = \"sk_test_67890\""));
        assert!(content.contains(&format!("agent_id = \"{}\"", new_agent_id)));
        assert!(content.contains("url = \"https://example.com\""));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_file_permissions_are_secure() {
        use std::os::unix::fs::PermissionsExt;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let agent_id = Uuid::parse_str("00000000-0000-0000-0000-000000000123").unwrap();
        save_api_key_to_config("sk_test_12345", agent_id, path)
            .await
            .unwrap();

        // Check permissions
        let metadata = fs::metadata(path).await.unwrap();
        let permissions = metadata.permissions();
        let mode = permissions.mode();

        // Should be 0600 (owner read/write only)
        assert_eq!(mode & 0o777, 0o600);
    }
}
