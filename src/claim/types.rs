//! Types for agent claiming workflow

use crate::openapi;
use uuid::Uuid;

impl openapi::AgentSelfRegistration {
    /// Create a new agent registration
    pub fn new(
        agent_id: Uuid,
        claim_token_hash: String,
        hostname: String,
        ip_addresses: Vec<openapi::AgentNetworkInterface>,
    ) -> Self {
        Self {
            agent_id,
            claim_token_hash,
            hostname,
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            ip_addresses,
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

        let registration = openapi::AgentSelfRegistration::new(
            agent_id,
            token_hash.clone(),
            hostname.clone(),
            vec![],
        );

        assert_eq!(registration.agent_id, agent_id);
        assert_eq!(registration.claim_token_hash, token_hash);
        assert_eq!(registration.hostname, hostname);
        assert!(!registration.agent_version.is_empty());
        assert!(registration.ip_addresses.is_empty());
    }

    #[test]
    fn test_agent_registration_with_ip_addresses() {
        use openapi::{AgentNetworkInterface, IpAddressFamily};

        let agent_id = Uuid::now_v7();
        let ip_addresses = vec![
            AgentNetworkInterface {
                ip: "192.168.1.10".to_string(),
                iface: "eth0".to_string(),
                family: IpAddressFamily::Ipv4,
                recommended: true,
            },
            AgentNetworkInterface {
                ip: "10.0.0.5".to_string(),
                iface: "eth1".to_string(),
                family: IpAddressFamily::Ipv4,
                recommended: false,
            },
        ];

        let registration = openapi::AgentSelfRegistration::new(
            agent_id,
            "hash".to_string(),
            "host".to_string(),
            ip_addresses.clone(),
        );

        assert_eq!(registration.ip_addresses.len(), 2);
        assert!(registration.ip_addresses[0].recommended);
        assert_eq!(registration.ip_addresses[0].ip, "192.168.1.10");
        assert_eq!(registration.ip_addresses[1].ip, "10.0.0.5");
    }
}
