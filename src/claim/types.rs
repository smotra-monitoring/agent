//! Types for agent claiming workflow

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Agent self-registration request
#[derive(Debug, Clone, Serialize)]
pub struct AgentRegistration {
    #[serde(rename = "agentId")]
    pub agent_id: Uuid,

    #[serde(rename = "claimTokenHash")]
    pub claim_token_hash: String,

    pub hostname: String,

    #[serde(rename = "agentVersion")]
    pub agent_version: String,
}

impl AgentRegistration {
    /// Create a new agent registration
    pub fn new(agent_id: Uuid, claim_token_hash: String, hostname: String) -> Self {
        Self {
            agent_id,
            claim_token_hash,
            hostname,
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Response from agent registration
#[derive(Debug, Clone, Deserialize)]
pub struct RegistrationResponse {
    pub status: String,

    #[serde(rename = "pollUrl")]
    pub poll_url: String,

    #[serde(rename = "claimUrl")]
    pub claim_url: String,

    #[serde(rename = "expiresAt")]
    pub expires_at: DateTime<Utc>,
}

/// Claim status response (pending)
#[derive(Debug, Clone, Deserialize)]
pub struct ClaimStatusPending {
    pub status: String,

    #[serde(rename = "expiresAt")]
    pub expires_at: DateTime<Utc>,
}

/// Claim status response (claimed)
#[derive(Debug, Clone, Deserialize)]
pub struct ClaimStatusClaimed {
    pub status: String,

    #[serde(rename = "apiKey")]
    pub api_key: String,

    #[serde(rename = "configUrl")]
    pub config_url: String,
}

/// Claim status enum
#[derive(Debug, Clone)]
pub enum ClaimStatus {
    Pending(ClaimStatusPending),
    Claimed(ClaimStatusClaimed),
}

/// Result of successful agent claiming workflow
///
/// Contains the data that needs to be persisted after claiming:
/// - API key for server authentication
/// - Agent ID (may be newly generated or existing)
#[derive(Debug, Clone)]
pub struct ClaimResult {
    /// API key for server authentication
    pub api_key: String,

    /// Agent ID (newly generated or existing)
    pub agent_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claim_result_creation() {
        let agent_id = Uuid::now_v7();
        let api_key = "sk_test_claim_result_123".to_string();

        let result = ClaimResult {
            api_key: api_key.clone(),
            agent_id,
        };

        assert_eq!(result.api_key, api_key);
        assert_eq!(result.agent_id, agent_id);
    }

    #[test]
    fn test_claim_result_clone() {
        let agent_id = Uuid::now_v7();
        let result = ClaimResult {
            api_key: "test_key".to_string(),
            agent_id,
        };

        let cloned = result.clone();
        assert_eq!(result.api_key, cloned.api_key);
        assert_eq!(result.agent_id, cloned.agent_id);
    }

    #[test]
    fn test_agent_registration_creation() {
        let agent_id = Uuid::now_v7();
        let token_hash = "hash123".to_string();
        let hostname = "test-host".to_string();

        let registration = AgentRegistration::new(agent_id, token_hash.clone(), hostname.clone());

        assert_eq!(registration.agent_id, agent_id);
        assert_eq!(registration.claim_token_hash, token_hash);
        assert_eq!(registration.hostname, hostname);
        assert!(!registration.agent_version.is_empty());
    }
}
