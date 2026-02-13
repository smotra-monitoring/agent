//! Polling for claim status

use crate::claim::types::{ClaimStatus, ClaimStatusClaimed, ClaimStatusPending};
use crate::error::{Error, Result};
use reqwest::{Client, StatusCode};
use std::time::Duration;
use tracing::{error, info};

/// Poll for claim status
///
/// Polls the server periodically to check if the agent has been claimed.
/// Returns the API key when the agent is successfully claimed.
///
/// # Arguments
///
/// * `client` - HTTP client to use for requests
/// * `base_url` - Base URL of the server
/// * `agent_id` - Agent ID
/// * `poll_url` - URL to poll for claim status (from registration response)
/// * `poll_interval` - Interval between poll attempts
///
/// # Returns
///
/// * `Ok(Some(api_key))` - Agent was claimed, API key received
/// * `Ok(None)` - Claim expired or not found
/// * `Err(...)` - Network or other error
pub async fn poll_claim_status(
    client: &Client,
    poll_url: &str,
    poll_interval: Duration,
) -> Result<Option<String>> {
    info!("Starting claim status polling");
    info!("Poll URL: {}", poll_url);
    info!("Poll interval: {:?}", poll_interval);

    loop {
        match check_claim_status(client, poll_url).await? {
            ClaimStatus::Pending(pending) => {
                let now = chrono::Utc::now();
                let expires_in = pending.expires_at.signed_duration_since(now);

                if expires_in.num_seconds() <= 0 {
                    error!("Claim has expired (status: {})", pending.status);
                    return Ok(None);
                }

                let hours = expires_in.num_hours();
                let minutes = (expires_in.num_minutes() % 60).abs();
                let seconds = (expires_in.num_seconds() % 60).abs();

                info!(
                    "Status: {} (expires in {}:{:02}:{:02})",
                    pending.status, hours, minutes, seconds
                );

                tokio::time::sleep(poll_interval).await;
            }
            ClaimStatus::Claimed(claimed) => {
                info!("Agent claimed successfully (status: {}) !", claimed.status);
                return Ok(Some(claimed.api_key));
            }
        }
    }
}

/// Check claim status once
///
/// Makes a single request to check the claim status.
///
/// # Arguments
///
/// * `client` - HTTP client to use
/// * `url` - Full URL to check claim status
///
/// # Errors
///
/// Returns an error if:
/// * The HTTP request fails
/// * The server returns a 404 (claim not found/expired)
/// * The response cannot be parsed
async fn check_claim_status(client: &Client, url: &str) -> Result<ClaimStatus> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| Error::Network(format!("Failed to check claim status: {}", e)))?;

    match response.status() {
        StatusCode::OK => {
            // Try to parse as pending first
            let text = response
                .text()
                .await
                .map_err(|e| Error::Network(format!("Failed to read response: {}", e)))?;

            // Try to determine status by checking the JSON
            let json: serde_json::Value =
                serde_json::from_str(&text).map_err(Error::Serialization)?;

            match json.get("status").and_then(|s| s.as_str()) {
                Some("pending_claim") => {
                    let pending: ClaimStatusPending =
                        serde_json::from_str(&text).map_err(Error::Serialization)?;
                    Ok(ClaimStatus::Pending(pending))
                }
                Some("claimed") => {
                    let claimed: ClaimStatusClaimed =
                        serde_json::from_str(&text).map_err(Error::Serialization)?;
                    Ok(ClaimStatus::Claimed(claimed))
                }
                _ => Err(Error::Network(format!(
                    "Unknown claim status: {:?}",
                    json.get("status")
                ))),
            }
        }
        StatusCode::NOT_FOUND => {
            error!("Agent registration not found or expired");
            Err(Error::Network(
                "Agent registration not found or expired".to_string(),
            ))
        }
        status => {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            error!(
                "Polling failed with HTTP status code {}: {}",
                status, error_text
            );
            Err(Error::Network(format!(
                "Polling failed with HTTP status code {}: {}",
                status, error_text
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claim_status_pending_deserialization() {
        let json = r#"{
            "status": "pending_claim",
            "expiresAt": "2026-02-01T12:00:00Z"
        }"#;

        let pending: ClaimStatusPending = serde_json::from_str(json).unwrap();

        assert_eq!(pending.status, "pending_claim");
    }

    #[test]
    fn test_claim_status_claimed_deserialization() {
        let json = r#"{
            "status": "claimed",
            "apiKey": "sk_live_abc123",
            "configUrl": "/agent/123/configuration"
        }"#;

        let claimed: ClaimStatusClaimed = serde_json::from_str(json).unwrap();

        assert_eq!(claimed.status, "claimed");
        assert_eq!(claimed.api_key, "sk_live_abc123");
    }

    #[tokio::test]
    async fn test_claim_expired_handling() {
        use mockito::Server;
        use uuid::Uuid;

        let mut server = Server::new_async().await;
        let agent_id = Uuid::now_v7();

        // Mock returns 404 (expired)
        let _mock_expired = server
            .mock(
                "GET",
                format!("/v1/agent/{}/claim-status", agent_id).as_str(),
            )
            .with_status(404)
            .create_async()
            .await;

        let client = reqwest::Client::new();

        let result = poll_claim_status(
            &client,
            &format!("{}/v1/agent/{}/claim-status", server.url(), agent_id),
            std::time::Duration::from_millis(100),
        )
        .await;

        assert!(result.is_err());
    }
}
