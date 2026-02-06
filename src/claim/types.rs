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
