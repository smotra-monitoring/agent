//! Shared `reqwest` client construction.

use super::user_agent::user_agent;
use std::time::Duration;

/// Build the HTTP client used for all agent -> server requests.
pub(crate) fn build_client(
    timeout: Duration,
    verify_tls: bool,
) -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(user_agent())
        .timeout(timeout)
        .danger_accept_invalid_certs(!verify_tls)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn client_sends_user_agent_header() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .match_header("user-agent", user_agent())
            .create_async()
            .await;

        let client = build_client(Duration::from_secs(5), true).unwrap();
        client.get(server.url()).send().await.unwrap();

        mock.assert_async().await;
    }
}
