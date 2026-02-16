//! Display claim information to the user

use tracing::info;
use uuid::Uuid;

use crate::openapi;

impl openapi::AgentRegistrationResponse {
    /// Display claim information in a formatted box
    ///
    /// Shows the agent ID, claim token, and instructions for claiming the agent
    /// through the web interface.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - The generated agent ID
    /// * `claim_token` - The plain claim token (not the hash)
    /// * `claim_url` - URL where users can claim the agent
    /// * `expires_at` - When the claim token expires
    pub fn display_claim_info(&self, agent_id: Uuid, claim_token: &str) {
        let border_top = "╔══════════════════════════════════════════════════════════════╗";
        let border_bottom = "╚══════════════════════════════════════════════════════════════╝";
        let border_mid = "╠══════════════════════════════════════════════════════════════╣";
        let empty_line = "║                                                              ║";

        info!("");
        info!("{}", border_top);
        info!("║              Agent Registration Required                     ║");
        info!("{}", border_mid);
        info!("{}", empty_line);
        info!("║  Agent ID:    {:<42} ║", agent_id);
        info!("║  Claim Token: {:<42} ║", claim_token);
        info!("{}", empty_line);
        info!("║  To claim this agent:                                        ║");
        info!("║  1. Go to: {:<50} ║", self.claim_url);
        info!("║  2. Enter the Agent ID and Claim Token shown above          ║");
        info!("║  3. This agent will start automatically once claimed        ║");
        info!("{}", empty_line);

        let expires_at_tz = self
            .expires_at
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S %Z");
        let duration = self.expires_at.signed_duration_since(chrono::Utc::now());
        let hours = duration.num_hours();

        info!(
            "║  Claim expires: {} (in {} hours)     ║",
            expires_at_tz, hours
        );
        info!("{}", empty_line);
        info!("{}", border_bottom);
        info!("");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_claim_info_does_not_panic() {
        let agent_id = Uuid::now_v7();
        let claim_token = "test_token_123";
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

        let registration_response = openapi::AgentRegistrationResponse {
            poll_url: "/v1/agent/poll".to_string(),
            claim_url: "https://smotra.example.com/claim".to_string(),
            expires_at,
            status: openapi::RegistrationStatus::PendingClaim,
        };

        // Should not panic
        registration_response.display_claim_info(agent_id, claim_token);
    }
}
