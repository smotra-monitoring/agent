//! Integration tests for config hot-reload functionality
//!
//! Tests the complete config reload workflow including:
//! - File watching and reload triggers
//! - Agent config updates
//! - Validation and error handling

use smotra::{
    load_and_validate_config, Agent, Config, ConfigReloadManager, Endpoint, MonitoringConfig,
    ReloadTrigger,
};
use std::fs;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::sync::broadcast;

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
async fn test_config_reload_file_change() {
    // Create a temporary directory and config file
    let config_path = NamedTempFile::new().unwrap();

    // Write initial config
    let initial_config = config_with_endpoints(1, 60, 2);
    initial_config
        .save_to_file_secure(&config_path)
        .await
        .unwrap();

    // Load initial config and create agent
    let config = load_and_validate_config(config_path.path()).unwrap();
    assert_eq!(config.version, 1);
    assert_eq!(config.monitoring.interval_secs, 60);
    assert_eq!(config.endpoints.len(), 2);

    let agent = Agent::new(config);

    // Set up reload manager
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let mut reload_manager =
        ConfigReloadManager::new(config_path.path().to_path_buf(), shutdown_rx).unwrap();

    // Start watching
    reload_manager.start_watching_file().unwrap();

    let agent_clone = std::sync::Arc::new(agent);
    let agent_for_callback = agent_clone.clone();
    let config_path_for_callback = config_path.path().to_path_buf();

    // Spawn reload manager
    let reload_handle = tokio::spawn(async move {
        reload_manager
            .run(move |_trigger| {
                let agent = agent_for_callback.clone();
                let config_path = config_path_for_callback.clone();
                async move {
                    let new_config = load_and_validate_config(&config_path)?;
                    agent.reload_config(new_config)?;
                    Ok(())
                }
            })
            .await
    });

    // Give watcher time to set up
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Modify the config file
    let updated_config = config_with_endpoints(2, 120, 3);
    updated_config
        .save_to_file_secure(&config_path)
        .await
        .unwrap();

    // Wait for reload to process
    tokio::time::sleep(Duration::from_millis(550)).await;

    // Verify config was reloaded
    let current_config = agent_clone.config_clone();
    assert_eq!(current_config.version, 2);
    assert_eq!(current_config.monitoring.interval_secs, 120);
    assert_eq!(current_config.endpoints.len(), 3);

    // Shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(2), reload_handle).await;
}

#[tokio::test]
async fn test_config_reload_manual_trigger() {
    // Create a temporary directory and config file
    let config_path = NamedTempFile::new().unwrap();

    // Write initial config
    let initial_config = config_with_endpoints(1, 60, 2);
    initial_config
        .save_to_file_secure(&config_path)
        .await
        .unwrap();

    let config = load_and_validate_config(config_path.path()).unwrap();
    let agent = Agent::new(config);

    // Create a temporary directory and config file
    let config_path2 = NamedTempFile::new().unwrap();

    // Write initial config
    let initial_config2 = config_with_endpoints(5, 90, 4);
    initial_config2
        .save_to_file_secure(&config_path2)
        .await
        .unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let reload_manager =
        ConfigReloadManager::new(config_path2.path().to_path_buf(), shutdown_rx).unwrap();

    let reload_tx = reload_manager.reload_sender();

    let agent_arc = std::sync::Arc::new(agent);
    let agent_for_callback = agent_arc.clone();
    let config_path_for_callback = config_path2.path().to_path_buf();

    let reload_handle = tokio::spawn(async move {
        reload_manager
            .run(move |_trigger| {
                let agent = agent_for_callback.clone();
                let config_path = config_path_for_callback.clone();
                async move {
                    let new_config = load_and_validate_config(&config_path)?;
                    agent.reload_config(new_config)?;
                    Ok(())
                }
            })
            .await
    });

    // Manually trigger reload
    reload_tx.send(ReloadTrigger::Manual).unwrap();

    // Wait for reload
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify
    let current_config = agent_arc.config_clone();
    assert_eq!(current_config.version, 5);
    assert_eq!(current_config.monitoring.interval_secs, 90);
    assert_eq!(current_config.endpoints.len(), 4);

    // Shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(2), reload_handle).await;
}

#[tokio::test]
async fn test_config_reload_invalid_config() {
    // Create a temporary directory and config file
    let config_path = NamedTempFile::new().unwrap();

    // Write initial config
    let initial_config = config_with_endpoints(1, 60, 2);
    initial_config
        .save_to_file_secure(&config_path)
        .await
        .unwrap();

    let config = load_and_validate_config(config_path.path()).unwrap();
    let agent = Agent::new(config.clone());

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let reload_manager =
        ConfigReloadManager::new(config_path.path().to_path_buf(), shutdown_rx).unwrap();

    let agent_arc = std::sync::Arc::new(agent);
    let agent_for_callback = agent_arc.clone();
    let config_path_for_callback = config_path.path().to_path_buf();

    let reload_handle = tokio::spawn(async move {
        reload_manager
            .run(move |_trigger| {
                let agent = agent_for_callback.clone();
                let config_path = config_path_for_callback.clone();
                async move {
                    let new_config = load_and_validate_config(&config_path)?;
                    agent.reload_config(new_config)?;
                    Ok(())
                }
            })
            .await
    });

    // Write invalid config (interval_secs = 0)
    let invalid_config = config_with_endpoints(2, 0, 1); // 0 is invalid
    invalid_config
        .save_to_file_secure(&config_path)
        .await
        .unwrap();

    // Wait for reload attempt
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify old config is still in effect
    let current_config = agent_arc.config_clone();
    assert_eq!(current_config.version, 1); // Should still be version 1
    assert_eq!(current_config.monitoring.interval_secs, 60); // Should still be 60

    // Shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(2), reload_handle).await;
}

#[tokio::test]
async fn test_config_reload_malformed_toml() {
    // Create a temporary directory and config file
    let config_path = NamedTempFile::new().unwrap();

    // Write initial config
    let initial_config = config_with_endpoints(1, 60, 1);
    initial_config
        .save_to_file_secure(&config_path)
        .await
        .unwrap();

    let config = load_and_validate_config(config_path.path()).unwrap();
    let agent = Agent::new(config.clone());

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let reload_manager =
        ConfigReloadManager::new(config_path.path().to_path_buf(), shutdown_rx).unwrap();

    let reload_tx = reload_manager.reload_sender();

    let agent_arc = std::sync::Arc::new(agent);
    let agent_for_callback = agent_arc.clone();
    let config_path_for_callback = config_path.path().to_path_buf();

    let reload_handle = tokio::spawn(async move {
        reload_manager
            .run(move |_trigger| {
                let agent = agent_for_callback.clone();
                let config_path = config_path_for_callback.clone();
                async move {
                    // This will fail due to malformed TOML
                    let new_config = load_and_validate_config(&config_path)?;
                    agent.reload_config(new_config)?;
                    Ok(())
                }
            })
            .await
    });

    // Write malformed TOML
    fs::write(&config_path, "invalid [ toml content").unwrap();

    // Trigger reload
    reload_tx.send(ReloadTrigger::Manual).unwrap();

    // Wait for reload attempt
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify old config is still in effect
    let current_config = agent_arc.config_clone();
    assert_eq!(current_config.version, 1);
    assert_eq!(current_config.monitoring.interval_secs, 60);

    // Shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(2), reload_handle).await;
}

#[tokio::test]
async fn test_multiple_config_reloads() {
    // Create a temporary directory and config file
    let config_path = NamedTempFile::new().unwrap();

    // Write initial config
    let initial_config = config_with_endpoints(1, 60, 2);
    initial_config
        .save_to_file_secure(&config_path)
        .await
        .unwrap();

    let config = load_and_validate_config(config_path.path()).unwrap();
    let agent = Agent::new(config);

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let mut reload_manager =
        ConfigReloadManager::new(config_path.path().to_path_buf(), shutdown_rx).unwrap();

    reload_manager.start_watching_file().unwrap();

    let reload_tx = reload_manager.reload_sender();

    let agent_arc = std::sync::Arc::new(agent);
    let agent_for_callback = agent_arc.clone();
    let config_path_for_callback = config_path.path().to_path_buf();

    let reload_handle = tokio::spawn(async move {
        reload_manager
            .run(move |_trigger| {
                let agent = agent_for_callback.clone();
                let config_path = config_path_for_callback.clone();
                async move {
                    let new_config = load_and_validate_config(&config_path)?;
                    agent.reload_config(new_config)?;
                    Ok(())
                }
            })
            .await
    });

    // Perform multiple reloads
    for i in 2..=5 {
        let updated_config = config_with_endpoints(i, 60 + (i as u64 * 10), i as usize);
        updated_config
            .save_to_file_secure(&config_path)
            .await
            .unwrap();

        // Do not reload manually, rely on file watcher to trigger reload
        // File debounce timer is 500ms, so current config will be reloaded after the loop is over.
        tokio::time::sleep(Duration::from_millis(50)).await;

        let current_config = agent_arc.config_clone();
        assert_eq!(current_config.version, 1);
        assert_eq!(current_config.monitoring.interval_secs, 60);
    }

    // Wait for file debounce timer to expire and reload will be triggered
    tokio::time::sleep(Duration::from_millis(550)).await;
    let current_config = agent_arc.config_clone();
    assert_eq!(current_config.version, 5);
    assert_eq!(current_config.monitoring.interval_secs, 110);

    // Perform multiple reloads
    for i in 2..=5 {
        let updated_config = config_with_endpoints(i, 60 + (i as u64 * 10), i as usize);
        updated_config
            .save_to_file_secure(&config_path)
            .await
            .unwrap();

        // Reload manually
        // File debounce timer is 500ms, speed it up by triggering manual reload.
        reload_tx.send(ReloadTrigger::Manual).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let current_config = agent_arc.config_clone();
        assert_eq!(current_config.version, i);
        assert_eq!(
            current_config.monitoring.interval_secs,
            60 + (i as u64 * 10)
        );
    }

    // Shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(2), reload_handle).await;
}

#[tokio::test]
async fn test_reload_trigger_variants() {
    // Create a temporary directory and config file
    let config_path = NamedTempFile::new().unwrap();

    // Write initial config
    let initial_config = config_with_endpoints(1, 60, 1);
    initial_config
        .save_to_file_secure(&config_path)
        .await
        .unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let reload_manager =
        ConfigReloadManager::new(config_path.path().to_path_buf(), shutdown_rx).unwrap();

    let reload_tx = reload_manager.reload_sender();

    let reload_handle = tokio::spawn(async move {
        reload_manager
            .run(move |trigger| {
                // Just log the trigger variants, testing that they're all accepted
                println!("Received trigger: {:?}", trigger);
                async move { Ok(()) }
            })
            .await
    });

    // Test different trigger types
    reload_tx
        .send(ReloadTrigger::FileChange(config_path.path().to_path_buf()))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    reload_tx.send(ReloadTrigger::Signal).unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    reload_tx.send(ReloadTrigger::Manual).unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    reload_tx
        .send(ReloadTrigger::ServerVersionChange(3))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(2), reload_handle).await;
}
