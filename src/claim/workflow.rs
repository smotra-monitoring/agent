//! Agent claiming workflow orchestration

use super::registration::register_with_retry;
use crate::claim::{
    polling::poll_claim_status,
    token::{generate_claim_token, hash_claim_token},
    types::AgentCredentials,
};
use crate::openapi::AgentSelfRegistration;
use crate::{Config, Error, Result};
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
}

impl<'a> Claim<'a> {
    /// Create a new claiming workflow
    ///
    /// # Arguments
    ///
    /// * `config` - Current agent configuration
    /// * `config_path` - Path to the configuration file
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    /// Run the claiming workflow
    ///
    /// # Returns
    ///
    /// `ClaimResult` containing the API key and agent ID once claimed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Server URL is not configured
    /// - Registration fails after all retries
    /// - Claim expires before being completed
    pub async fn run(&self) -> Result<AgentCredentials> {
        let server_url = &self.config.server.url;

        // Generate agent ID if not set (nil UUID means unregistered)
        let agent_id = if self.config.agent_id == Uuid::nil() {
            Uuid::now_v7()
        } else {
            self.config.agent_id
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
        let registration = AgentSelfRegistration::new(agent_id, claim_token_hash, hostname);

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
        registration_response.display_claim_info(agent_id, &claim_token);

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

                Ok(AgentCredentials { api_key, agent_id })
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

    #[test]
    fn test_claim_creation() {
        let test_agent_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let config = Config {
            agent_id: test_agent_id,
            agent_name: "Test Agent".to_string(),
            ..Default::default()
        };

        let claim = Claim::new(&config);

        // Just verify we can create the struct
        assert_eq!(claim.config.agent_id, test_agent_id);
    }

    #[test]
    fn test_claim_generates_agent_id_when_nil() {
        let config = Config {
            agent_id: Uuid::nil(),
            agent_name: "Test Agent".to_string(),
            ..Default::default()
        };

        let claim = Claim::new(&config);

        // Verify config has nil UUID initially
        assert_eq!(claim.config.agent_id, Uuid::nil());

        // Note: Full workflow test with agent ID generation would be in integration tests
        // as it requires HTTP server mock. The run() method will generate a new UUID
        // when agent_id is nil.
    }
}
