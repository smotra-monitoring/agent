//! Types for agent claiming workflow

use crate::openapi;
use uuid::Uuid;

impl openapi::AgentSelfRegistration {
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

/// Claim status enum
#[derive(Debug, Clone)]
pub enum ClaimStatus {
    Pending(openapi::ClaimStatusPending),
    Claimed(openapi::ClaimStatusClaimed),
}

/// Result of successful agent claiming workflow
///
/// Contains the data that needs to be persisted after claiming:
/// - API key for server authentication
/// - Agent ID (may be newly generated or existing)
#[derive(Debug, Clone)]
pub struct AgentCredentials {
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

        let result = AgentCredentials {
            api_key: api_key.clone(),
            agent_id,
        };

        assert_eq!(result.api_key, api_key);
        assert_eq!(result.agent_id, agent_id);
    }

    #[test]
    fn test_claim_result_clone() {
        let agent_id = Uuid::now_v7();
        let result = AgentCredentials {
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

        let registration =
            openapi::AgentSelfRegistration::new(agent_id, token_hash.clone(), hostname.clone());

        assert_eq!(registration.agent_id, agent_id);
        assert_eq!(registration.claim_token_hash, token_hash);
        assert_eq!(registration.hostname, hostname);
        assert!(!registration.agent_version.is_empty());
    }
}
