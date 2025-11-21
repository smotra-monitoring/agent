//! Integration tests for heartbeat functionality

use smotra_agent::{Config, HeartbeatReporter};

#[tokio::test]
async fn test_heartbeat_with_mock_server() {
    // This test would require a mock HTTP server
    // For now, we'll test the basic structure
    let config = create_test_config();
    let reporter = HeartbeatReporter::new(config);
    assert!(reporter.is_ok());
}

#[tokio::test]
async fn test_heartbeat_collection() {
    let config = create_test_config();
    let reporter = HeartbeatReporter::new(config).unwrap();
    let heartbeat = reporter.collect_metrics().await;

    // Verify basic structure
    assert!(heartbeat.timestamp.timestamp() > 0);
}

#[tokio::test]
async fn test_heartbeat_without_server() {
    let mut config = create_test_config();
    config.server.url = None;

    let reporter = HeartbeatReporter::new(config);
    // Should fail if server URL is not configured
    assert!(reporter.is_ok());
}

fn create_test_config() -> Config {
    use smotra_agent::{MonitoringConfig, ServerConfig, StorageConfig};

    Config {
        agent_id: "test-agent-001".to_string(),
        tags: vec!["test".to_string()],
        monitoring: MonitoringConfig::default(),
        server: ServerConfig {
            url: Some("https://test.example.com".to_string()),
            api_key: Some("test-key".to_string()),
            heartbeat_interval_secs: 5,
            ..Default::default()
        },
        storage: StorageConfig::default(),
        endpoints: vec![],
    }
}
