//! Integration tests for agent claiming workflow public API

use smotra_agent::save_api_key_to_config;
use smotra_agent::{Claim, Config};
use tempfile::NamedTempFile;
use uuid::Uuid;

#[tokio::test]
async fn test_save_api_key_updates_config() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();

    // Create initial config without API key
    let initial_config = r#"
version = 1
agent_id = ""
agent_name = "Test Agent"

[server]
url = "https://example.com"
"#;
    tokio::fs::write(path, initial_config).await.unwrap();

    // Save API key
    let agent_id = Uuid::now_v7();
    save_api_key_to_config("sk_test_12345", agent_id, path)
        .await
        .expect("Should save API key");

    // Read back and verify
    let content = tokio::fs::read_to_string(path).await.unwrap();

    assert!(content.contains("api_key = \"sk_test_12345\""));
    assert!(content.contains(&format!("agent_id = \"{}\"", agent_id)));
}

#[test]
fn test_claim_api_is_accessible() {
    // Verify we can create a Claim using the public API
    let config = Config::default();
    let temp_file = NamedTempFile::new().unwrap();

    let _claim = Claim::new(&config, temp_file.path());

    // The only way to use claiming is via the Claim object
    // Users would call: claim.run().await
}

#[tokio::test]
async fn test_claim_workflow_with_mock_server() {
    use mockito::Server;

    let mut server = Server::new_async().await;

    // Create a config pointing to our mock server
    let temp_file = NamedTempFile::new().unwrap();

    let mut config = Config::default();
    config.server.url = server.url();
    config.server.verify_tls = false;
    config.server.claiming.max_registration_retries = 1;

    // Save config to file
    config.save_to_file(temp_file.path()).unwrap();

    // Mock registration endpoint - return error to fail quickly
    let _mock_register = server
        .mock("POST", "/v1/agent/register")
        .with_status(500)
        .create_async()
        .await;

    let claim = Claim::new(&config, temp_file.path());

    // This should fail because mock returns 500
    let result = claim.run().await;
    assert!(result.is_err());
}
