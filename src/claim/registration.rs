//! Agent registration logic

use crate::claim::types::{AgentRegistration, RegistrationResponse};
use crate::error::{Error, Result};
use reqwest::Client;
use std::time::Duration;
use tracing::{info, warn};

/// Register agent with retry logic and exponential backoff
///
/// Attempts to register the agent multiple times with exponential backoff
/// between retries.
///
/// # Arguments
///
/// * `client` - HTTP client to use for the request
/// * `base_url` - Base URL of the server
/// * `registration` - Agent registration data
/// * `max_retries` - Maximum number of retry attempts
///
/// # Errors
///
/// Returns an error if all retry attempts fail.
pub async fn register_with_retry(
    client: &Client,
    base_url: &str,
    registration: AgentRegistration,
    max_retries: u32,
) -> Result<RegistrationResponse> {
    let mut delay = Duration::from_secs(1);

    for attempt in 0..max_retries {
        match register_agent(client, base_url, registration.clone()).await {
            Ok(response) => return Ok(response),
            Err(e) if attempt < max_retries - 1 => {
                warn!(
                    "Registration attempt {} of {} failed: {}",
                    attempt + 1,
                    max_retries,
                    e
                );
                warn!("Retrying in {:?}...", delay);
                tokio::time::sleep(delay).await;
                delay *= 2; // Exponential backoff
            }
            Err(e) => return Err(e),
        }
    }

    unreachable!()
}

/// Register agent with the server
///
/// Sends a registration request to the server with the agent ID and claim token hash.
/// Returns the registration response with polling URL and claim URL.
///
/// # Arguments
///
/// * `client` - HTTP client to use for the request
/// * `base_url` - Base URL of the server (e.g., "https://api.smotra.net")
/// * `registration` - Agent registration data
///
/// # Errors
///
/// Returns an error if the HTTP request fails or the server returns an error status.
async fn register_agent(
    client: &Client,
    base_url: &str,
    registration: AgentRegistration,
) -> Result<RegistrationResponse> {
    let url = format!("{}/v1/agent/register", base_url);

    info!("Registering agent with server: {}", url);

    let response = client
        .post(&url)
        .json(&registration)
        .send()
        .await
        .map_err(|e| Error::Network(format!("Failed to send registration request: {}", e)))?;

    let status = response.status();

    if status.is_success() {
        let registration_response = response
            .json::<RegistrationResponse>()
            .await
            .map_err(|e| Error::Network(format!("Failed to parse registration response: {}", e)))?;

        info!("Registration successful");
        Ok(registration_response)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        Err(Error::Network(format!(
            "Registration failed with status {}: {}",
            status, error_text
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_agent_registration_serialization() {
        let agent_id = Uuid::now_v7();
        let registration =
            AgentRegistration::new(agent_id, "abc123".to_string(), "test-host".to_string());

        let json = serde_json::to_value(&registration).unwrap();

        assert_eq!(json["agentId"], agent_id.to_string());
        assert_eq!(json["claimTokenHash"], "abc123");
        assert_eq!(json["hostname"], "test-host");
        assert!(json["agentVersion"].is_string());
    }

    #[test]
    fn test_registration_response_deserialization() {
        let json = r#"{
            "status": "pending_claim",
            "pollUrl": "/agent/123/claim-status",
            "claimUrl": "https://example.com/claim",
            "expiresAt": "2026-02-01T12:00:00Z"
        }"#;

        let response: RegistrationResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.status, "pending_claim");
        assert_eq!(response.poll_url, "/agent/123/claim-status");
        assert_eq!(response.claim_url, "https://example.com/claim");
        assert_eq!(
            response.expires_at.to_rfc3339(),
            "2026-02-01T12:00:00+00:00"
        );
    }

    #[tokio::test]
    async fn test_registration_with_retry() {
        use mockito::Server;

        let mut server = Server::new_async().await;
        let agent_id = Uuid::now_v7();

        // First 2 attempts fail with 500
        let _mock_fail = server
            .mock("POST", "/v1/agent/register")
            .with_status(500)
            .expect(2)
            .create_async()
            .await;

        // Third attempt succeeds
        let _mock_success = server
            .mock("POST", "/v1/agent/register")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{
                    "status": "pending_claim",
                    "pollUrl": "/v1/agent/{}/claim-status",
                    "claimUrl": "https://example.com/claim",
                    "expiresAt": "2026-02-01T12:00:00Z"
                }}"#,
                agent_id
            ))
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let registration =
            AgentRegistration::new(agent_id, "test_hash".to_string(), "test-host".to_string());

        let result = register_with_retry(&client, &server.url(), registration, 3).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_idempotent_registration() {
        use mockito::Server;

        let mut server = Server::new_async().await;
        let agent_id = Uuid::now_v7();

        // Both requests should succeed
        let _mock_register = server
            .mock("POST", "/v1/agent/register")
            .with_status(200) // 200 for idempotent update
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{
                    "status": "pending_claim",
                    "pollUrl": "/v1/agent/{}/claim-status",
                    "claimUrl": "https://example.com/claim",
                    "expiresAt": "2026-02-01T12:00:00Z"
                }}"#,
                agent_id
            ))
            .expect(2)
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let registration =
            AgentRegistration::new(agent_id, "test_hash".to_string(), "test-host".to_string());

        // First registration
        let result1 = register_agent(&client, &server.url(), registration.clone()).await;
        assert!(result1.is_ok());

        // Second registration (idempotent)
        let result2 = register_agent(&client, &server.url(), registration.clone()).await;
        assert!(result2.is_ok());

        assert_eq!(
            result1.as_ref().unwrap().claim_url,
            result2.as_ref().unwrap().claim_url
        );
        assert_eq!(
            result1.as_ref().unwrap().poll_url,
            result2.as_ref().unwrap().poll_url
        );
        assert_eq!(
            result1.as_ref().unwrap().status,
            result2.as_ref().unwrap().status
        );
    }
}
