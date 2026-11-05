use smotra::{Agent, AgentConfig, ServerConfig, StorageConfig};
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;
use uuid::Uuid;

/// Drift remediation end to end: the heartbeat reports a newer desired version, the agent
/// fetches the config, saves it, and the file watcher applies it to the running agent.
#[tokio::test]
async fn agent_applies_server_config_after_heartbeat_reports_newer_version() {
    let mut server = mockito::Server::new_async().await;
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let local = AgentConfig {
        version: 1,
        agent_id: Uuid::now_v7(),
        agent_name: "local".to_string(),
        server: ServerConfig {
            url: server.url(),
            api_key: Some("sk_local".to_string()),
            ..Default::default()
        },
        storage: StorageConfig {
            cache_dir: dir.path().join("cache").to_string_lossy().into_owned(),
            ..Default::default()
        },
        ..Default::default()
    };
    local.save_to_file_secure(&config_path).await.unwrap();

    // The server sends neither the api_key nor the agent's host-specific cache_dir.
    let mut remote = local.clone();
    remote.version = 2;
    remote.agent_name = "from-server".to_string();
    remote.server.api_key = None;
    remote.storage.cache_dir = "./server-cache".to_string();

    server
        .mock(
            "POST",
            format!("/agent/{}/heartbeat", local.agent_id).as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"config_version_desired": 2}"#)
        .create_async()
        .await;
    let configuration = server
        .mock(
            "GET",
            format!("/agent/{}/configuration", local.agent_id).as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&remote).unwrap())
        .create_async()
        .await;

    let agent = Arc::new(Agent::new(config_path.clone()).unwrap());
    assert_eq!(agent.status().config_version, 1);

    let running = {
        let agent = Arc::clone(&agent);
        tokio::spawn(async move { agent.start().await })
    };

    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while agent.status().config_version != 2 && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    agent.stop().unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(5), running).await;

    assert_eq!(
        agent.status().config_version,
        2,
        "agent should run the server's config version"
    );
    configuration.assert_async().await;

    let saved = AgentConfig::from_file(&config_path).unwrap();
    assert_eq!(saved.version, 2);
    assert_eq!(saved.agent_name, "from-server");
    assert_eq!(saved.server.api_key.as_deref(), Some("sk_local"));
    assert_eq!(saved.storage.cache_dir, local.storage.cache_dir);
}
