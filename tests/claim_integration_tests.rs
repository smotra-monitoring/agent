//! Integration tests for agent claiming workflow public API

use smotra::{Claim, Config};
use tempfile::NamedTempFile;

#[test]
fn test_claim_api_is_accessible() {
    // Verify we can create a Claim using the public API
    let config = Config::default();

    let _claim = Claim::new(&config);

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
    config.save_to_file_secure(temp_file.path()).await.unwrap();

    // Mock registration endpoint - return error to fail quickly
    let _mock_register = server
        .mock("POST", "/v1/agent/register")
        .with_status(500)
        .create_async()
        .await;

    let claim = Claim::new(&config);

    // This should fail because mock returns 500
    let result = claim.run().await;
    assert!(result.is_err());
}
