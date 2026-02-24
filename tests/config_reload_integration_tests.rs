//! Integration tests for config hot-reload functionality
//!
//! Tests the complete config reload workflow including:
//! - File watching and reload triggers
//! - Agent config updates
//! - Validation and error handling

use smotra::{test_helpers, Agent, Config, Endpoint, MonitoringConfig};
use std::fs;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::sync::{broadcast, mpsc};

/// Helper to create a valid config file content
fn config_with_endpoints(version: u32, interval_secs: u64, endpoints_count: usize) -> Config {
    let endpoints: Vec<Endpoint> = (1..=endpoints_count)
        .map(|i| Endpoint::new(format!("192.168.1.{}", i)))
        .collect();

    Config {
        version,
        agent_id: uuid::Uuid::new_v4(),
        monitoring: MonitoringConfig {
            interval_secs,
            ..MonitoringConfig::default()
        },
        endpoints,
        ..Config::default()
    }
}

#[tokio::test]
async fn test_config_reload_manual_trigger() {
    // Create config files
    let config_path = NamedTempFile::new().unwrap();
    let initial_config = config_with_endpoints(1, 60, 2);
    initial_config
        .save_to_file_secure(&config_path)
        .await
        .unwrap();

    let agent = Agent::new(config_path.path().to_path_buf()).unwrap();

    // Override with a different config file for reload testing
    let config_path2 = NamedTempFile::new().unwrap();
    let updated_config = config_with_endpoints(5, 90, 4);
    updated_config
        .save_to_file_secure(&config_path2)
        .await
        .unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (trigger_tx, trigger_rx) = test_helpers::create_reload_trigger_channel();
    let (config_tx, mut config_rx) = mpsc::channel(1);

    // Spawn the test reload handler
    let config_path2_clone = config_path2.path().to_path_buf();
    let reload_handle = tokio::spawn(async move {
        test_helpers::run_hot_reload_with_trigger_channel(
            config_path2_clone,
            config_tx,
            trigger_rx,
            shutdown_rx,
        )
        .await
    });

    // Manually trigger reload
    trigger_tx
        .send(test_helpers::ReloadTrigger::Manual)
        .unwrap();

    // Wait for the reloaded config
    let new_config = tokio::time::timeout(Duration::from_millis(500), config_rx.recv())
        .await
        .expect("Should receive config within timeout")
        .expect("Should receive Some(config)");

    // Apply the config to agent
    agent.reload_config(new_config).unwrap();

    // Verify
    let current_config = agent.config_clone();
    assert_eq!(current_config.version, 5);
    assert_eq!(current_config.monitoring.interval_secs, 90);
    assert_eq!(current_config.endpoints.len(), 4);

    // Shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(1), reload_handle)
        .await
        .expect("Hot reload task should shutdown cleanly");
}

#[tokio::test]
async fn test_config_reload_invalid_config() {
    // Create config file
    let config_path = NamedTempFile::new().unwrap();
    let initial_config = config_with_endpoints(1, 60, 2);
    initial_config
        .save_to_file_secure(&config_path)
        .await
        .unwrap();

    let agent = Agent::new(config_path.path().to_path_buf()).unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (trigger_tx, trigger_rx) = test_helpers::create_reload_trigger_channel();
    let (config_tx, mut config_rx) = mpsc::channel(1);

    // Spawn reload handler
    let config_path_clone = config_path.path().to_path_buf();
    let reload_handle = tokio::spawn(async move {
        test_helpers::run_hot_reload_with_trigger_channel(
            config_path_clone,
            config_tx,
            trigger_rx,
            shutdown_rx,
        )
        .await
    });

    // Write invalid config (interval_secs = 0)
    let invalid_config = config_with_endpoints(2, 0, 1); // 0 is invalid
    invalid_config
        .save_to_file_secure(&config_path)
        .await
        .unwrap();

    // Trigger reload
    trigger_tx
        .send(test_helpers::ReloadTrigger::Manual)
        .unwrap();

    // Try to receive new config - should not get one within timeout because validation fails
    let result = tokio::time::timeout(Duration::from_millis(200), config_rx.recv()).await;

    assert!(
        result.is_err(),
        "Timeout should occur because invalid config should not be sent through channel"
    );

    // Verify old config is still in effect (no reload happened)
    let current_config = agent.config_clone();
    assert_eq!(current_config.version, 1);
    assert_eq!(current_config.monitoring.interval_secs, 60);

    // Shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(1), reload_handle)
        .await
        .expect("Hot reload task should shutdown cleanly");
}

#[tokio::test]
async fn test_config_reload_malformed_toml() {
    // Create config file
    let config_path = NamedTempFile::new().unwrap();
    let initial_config = config_with_endpoints(1, 60, 1);
    initial_config
        .save_to_file_secure(&config_path)
        .await
        .unwrap();

    let agent = Agent::new(config_path.path().to_path_buf()).unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (trigger_tx, trigger_rx) = test_helpers::create_reload_trigger_channel();
    let (config_tx, mut config_rx) = mpsc::channel(1);

    // Spawn reload handler
    let config_path_clone = config_path.path().to_path_buf();
    let reload_handle = tokio::spawn(async move {
        test_helpers::run_hot_reload_with_trigger_channel(
            config_path_clone,
            config_tx,
            trigger_rx,
            shutdown_rx,
        )
        .await
    });

    // Write malformed TOML
    fs::write(&config_path, "invalid [ toml content").unwrap();

    // Trigger reload
    trigger_tx
        .send(test_helpers::ReloadTrigger::Manual)
        .unwrap();

    // Should not receive new config because parsing fails
    let _result = tokio::time::timeout(Duration::from_millis(200), config_rx.recv()).await;

    assert!(
        _result.is_err(),
        "Timeout should occur because malformed config should not be sent through channel"
    );

    // Verify old config is still in effect
    let current_config = agent.config_clone();
    assert_eq!(current_config.version, 1);
    assert_eq!(current_config.monitoring.interval_secs, 60);

    // Shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(1), reload_handle).await;
}

#[tokio::test]
async fn test_multiple_config_reloads() {
    // Create config file
    let config_path = NamedTempFile::new().unwrap();
    let initial_config = config_with_endpoints(1, 60, 2);
    initial_config
        .save_to_file_secure(&config_path)
        .await
        .unwrap();

    let agent = Agent::new(config_path.path().to_path_buf()).unwrap();
    let agent_arc = std::sync::Arc::new(agent);

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (trigger_tx, trigger_rx) = test_helpers::create_reload_trigger_channel();
    let (config_tx, mut config_rx) = mpsc::channel(1);

    // Spawn reload handler
    let config_path_clone = config_path.path().to_path_buf();
    let agent_for_task = agent_arc.clone();
    let reload_handle = tokio::spawn(async move {
        let reload_task = tokio::spawn(async move {
            test_helpers::run_hot_reload_with_trigger_channel(
                config_path_clone,
                config_tx,
                trigger_rx,
                shutdown_rx,
            )
            .await
        });

        // Apply configs as they come through the channel
        while let Some(new_config) = config_rx.recv().await {
            let _ = agent_for_task.reload_config(new_config);
        }

        let _ = reload_task.await;
    });

    // Perform multiple reloads
    for i in 2..=5 {
        let updated_config = config_with_endpoints(i, 60 + (i as u64 * 10), i as usize);
        updated_config
            .save_to_file_secure(&config_path)
            .await
            .unwrap();

        // Trigger manual reload
        trigger_tx
            .send(test_helpers::ReloadTrigger::Manual)
            .unwrap();

        // Wait for reload to be applied
        tokio::time::sleep(Duration::from_millis(100)).await;

        let current_config = agent_arc.config_clone();
        assert_eq!(current_config.version, i);
        assert_eq!(
            current_config.monitoring.interval_secs,
            60 + (i as u64 * 10)
        );
    }

    // Shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(1), reload_handle).await;
}

#[tokio::test]
async fn test_reload_trigger_variants() {
    // Create config file
    let config_path = NamedTempFile::new().unwrap();
    let initial_config = config_with_endpoints(1, 60, 1);
    initial_config
        .save_to_file_secure(&config_path)
        .await
        .unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (trigger_tx, trigger_rx) = test_helpers::create_reload_trigger_channel();
    let (reload_tx, mut reload_rx) = mpsc::channel(1);

    // Spawn reload handler
    let config_path_clone = config_path.path().to_path_buf();
    let reload_handle = tokio::spawn(async move {
        test_helpers::run_hot_reload_with_trigger_channel(
            config_path_clone,
            reload_tx,
            trigger_rx,
            shutdown_rx,
        )
        .await
    });

    // Test different trigger types - all should work
    trigger_tx
        .send(test_helpers::ReloadTrigger::FileChange(
            config_path.path().to_path_buf(),
        ))
        .unwrap();
    let config = tokio::time::timeout(Duration::from_millis(100), reload_rx.recv())
        .await
        .expect("Should receive reload trigger within timeout");

    assert!(config.is_some(), "Expected config received");

    trigger_tx
        .send(test_helpers::ReloadTrigger::Signal)
        .unwrap();
    let config = tokio::time::timeout(Duration::from_millis(100), reload_rx.recv())
        .await
        .expect("Should receive reload trigger within timeout");

    assert!(config.is_some(), "Expected config received");

    trigger_tx
        .send(test_helpers::ReloadTrigger::Manual)
        .unwrap();
    let config = tokio::time::timeout(Duration::from_millis(100), reload_rx.recv())
        .await
        .expect("Should receive reload trigger within timeout");

    assert!(config.is_some(), "Expected config received");

    trigger_tx
        .send(test_helpers::ReloadTrigger::ServerVersionChange(3))
        .unwrap();
    let config = tokio::time::timeout(Duration::from_millis(100), reload_rx.recv())
        .await
        .expect("Should receive reload trigger within timeout");

    assert!(config.is_some(), "Expected config received");

    // Shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(1), reload_handle).await;
}
