//! Agent claiming workflow orchestration

use super::registration::register_with_retry;
use crate::agent_config::server_config::save_api_key_to_config;
use crate::claim::{
    display::display_claim_info,
    polling::poll_claim_status,
    token::{generate_claim_token, hash_claim_token},
    types::AgentRegistration,
};
use crate::{Config, Error, Result};
use std::path::Path;
use tracing::{error, info};
use uuid::Uuid;

/// Agent claiming workflow orchestrator
///
/// Handles the complete agent claiming workflow:
/// 1. Generate agent ID and claim token
/// 2. Register with server
/// 3. Display claim information to user
/// 4. Poll for claim status
/// 5. Save API key when claimed
pub struct Claim<'a> {
    config: &'a Config,
    config_path: &'a Path,
}

impl<'a> Claim<'a> {
    /// Create a new claiming workflow
    ///
    /// # Arguments
    ///
    /// * `config` - Current agent configuration
    /// * `config_path` - Path to the configuration file
    pub fn new(config: &'a Config, config_path: &'a Path) -> Self {
        Self {
            config,
            config_path,
        }
    }

    /// Run the claiming workflow
    ///
    /// # Returns
    ///
    /// The API key once the agent is claimed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Server URL is not configured
    /// - Registration fails after all retries
    /// - Claim expires before being completed
    /// - Failed to save API key to configuration
    pub async fn run(&self) -> Result<String> {
        let server_url = &self.config.server.url;

        // Generate agent ID if not set
        let agent_id =
            if self.config.agent_id.is_empty() || self.config.agent_id == Uuid::nil().to_string() {
                Uuid::now_v7()
            } else {
                self.config
                    .agent_id
                    .parse()
                    .unwrap_or_else(|_| Uuid::now_v7())
            };

        info!("Agent ID: {}", agent_id);

        // Generate claim token
        let claim_token = generate_claim_token();
        let claim_token_hash = hash_claim_token(&claim_token);

        info!("Claim token generated (hash will be sent to server)");

        // Get hostname
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        // Create registration
        let registration = AgentRegistration::new(agent_id, claim_token_hash, hostname);

        // Create HTTP client
        let client = reqwest::Client::builder()
            .timeout(self.config.server.timeout())
            .danger_accept_invalid_certs(!self.config.server.verify_tls)
            .build()
            .map_err(|e| Error::Network(format!("Failed to create HTTP client: {}", e)))?;

        // Register with server (with retries)
        info!("Registering agent with server...");
        let registration_response = register_with_retry(
            &client,
            server_url,
            registration,
            self.config.server.claiming.max_registration_retries,
        )
        .await?;

        info!("Registration successful");

        // Display claim information
        display_claim_info(
            agent_id,
            &claim_token,
            &registration_response.claim_url,
            registration_response.expires_at,
        );

        info!("Waiting for agent to be claimed...");
        info!(
            "Polling claim status every {:?} (press Ctrl+C to cancel)",
            self.config.server.claiming.poll_interval()
        );

        // Poll for claim status
        let api_key = poll_claim_status(
            &client,
            format!("{}{}", server_url, &registration_response.poll_url).as_str(),
            self.config.server.claiming.poll_interval(),
        )
        .await?;

        match api_key {
            Some(api_key) => {
                info!("API key received");

                // Save to configuration
                save_api_key_to_config(&api_key, &agent_id.to_string(), self.config_path).await?;

                Ok(api_key)
            }
            None => {
                error!("Claim expired or cancelled");
                Err(Error::ClaimExpired)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_config::{MonitoringConfig, ServerConfig, StorageConfig};
    use tempfile::NamedTempFile;

    #[test]
    fn test_claim_creation() {
        let config = Config {
            version: 1,
            agent_id: "test-agent-001".to_string(),
            agent_name: "Test Agent".to_string(),
            tags: vec![],
            monitoring: MonitoringConfig::default(),
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            endpoints: vec![],
        };

        let temp_file = NamedTempFile::new().unwrap();
        let claim = Claim::new(&config, temp_file.path());

        // Just verify we can create the struct
        assert_eq!(claim.config.agent_id, "test-agent-001");
    }
}
