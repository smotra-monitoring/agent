//! Server reporting functionality

use crate::agent_config::sync_config_from_server;
use crate::core::AgentStatus;
use crate::error::{Error, Result};
use crate::openapi::AgentConfig;
use crate::reporter::HeartbeatReporter;
use chrono::Utc;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Heartbeats to skip before re-fetching a version that is already saved or failed to apply.
const SYNC_RETRY_AFTER_HEARTBEATS: u32 = 3;

/// Stops the same desired version from being fetched on every heartbeat.
///
/// After a save, the file watcher needs a moment to apply it; after a failure, retrying
/// every heartbeat would hammer the server.
#[derive(Debug, Default)]
struct SyncThrottle {
    last_attempted: Option<i64>,
    skipped: u32,
}

impl SyncThrottle {
    fn should_attempt(&mut self, desired_version: i64) -> bool {
        if self.last_attempted == Some(desired_version)
            && self.skipped < SYNC_RETRY_AFTER_HEARTBEATS
        {
            self.skipped += 1;
            return false;
        }

        self.last_attempted = Some(desired_version);
        self.skipped = 0;
        true
    }
}

/// Run the heartbeat loop in a separate task
///
/// Accepts a shared `Arc<RwLock<Config>>` so that config hot-reloads are
/// reflected in subsequent heartbeat payloads automatically.
/// Accepts a shared `Arc<RwLock<AgentStatus>>` so each heartbeat payload
/// includes the latest agent status snapshot.
///
/// When the server reports a newer desired config version, the new config is fetched
/// and saved to `config_path`; the file watcher then applies it.
pub async fn run_heartbeat(
    config: Arc<RwLock<AgentConfig>>,
    agent_status: Arc<RwLock<AgentStatus>>,
    config_path: PathBuf,
    mut agent_shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting heartbeat reporter");

    if !config.read().server.is_configured() {
        warn!("Server not configured, heartbeat disabled");
        return Ok(());
    }

    let heartbeat_reporter =
        HeartbeatReporter::new(Arc::clone(&config), Arc::clone(&agent_status))?;

    // Track the current interval so we can hot-reload it when config changes.
    let mut current_interval_duration = config.read().server.heartbeat_interval();
    let mut iv = interval(current_interval_duration);
    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut sync_throttle = SyncThrottle::default();

    loop {
        tokio::select! {
            _ = iv.tick() => {
                // Detect interval changes and recreate the timer.
                let new_interval = config.read().server.heartbeat_interval();
                if new_interval != current_interval_duration {
                    info!(
                        "Heartbeat interval changed from {:?} to {:?}, recreating timer",
                        current_interval_duration, new_interval
                    );
                    current_interval_duration = new_interval;
                    iv = interval(current_interval_duration);
                    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                }

                match heartbeat_reporter.send_heartbeat().await {
                    Ok(desired_version) => {
                        debug!("Heartbeat sent successfully");
                        agent_status.write().server_connected = true;
                        agent_status.write().reported_at = Utc::now();

                        if let Some(desired) = desired_version {
                            // Snapshot so the lock is not held across the network call.
                            let local = config.read().clone();
                            if local.version < desired && sync_throttle.should_attempt(desired) {
                                info!(
                                    "Server desired config version {} (current {}), fetching...",
                                    desired, local.version
                                );
                                if let Err(e) = sync_config_from_server(&local, &config_path).await {
                                    error!("Config sync failed: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        agent_status.write().server_connected = false;
                        agent_status.write().failed_report_count += 1;

                        // Log error but continue - heartbeats are best-effort
                        match &e {
                            Error::Authentication(_) => {
                                error!("Heartbeat authentication failed: {}", e);
                            }
                            Error::Network(_) => {
                                warn!("Heartbeat network error: {}", e);
                            }
                            _ => {
                                error!("Heartbeat failed: {}", e);
                            }
                        }
                    }
                }
            }
            _ = agent_shutdown_rx.recv() => {
                info!("Heartbeat reporter shutting down");
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openapi::ServerConfig;
    use std::time::Duration;
    use tempfile::NamedTempFile;
    use uuid::Uuid;

    mod sync_throttle_tests {
        use super::*;

        /// A new desired version is always attempted.
        #[test]
        fn first_attempt_is_allowed() {
            let mut throttle = SyncThrottle::default();
            assert!(throttle.should_attempt(2));
        }

        /// The same version is skipped for N heartbeats, then retried.
        #[test]
        fn same_version_is_skipped_then_retried() {
            let mut throttle = SyncThrottle::default();
            assert!(throttle.should_attempt(2));

            for _ in 0..SYNC_RETRY_AFTER_HEARTBEATS {
                assert!(!throttle.should_attempt(2));
            }
            assert!(
                throttle.should_attempt(2),
                "should retry after the skip window"
            );
            assert!(
                !throttle.should_attempt(2),
                "skip window restarts after a retry"
            );
        }

        /// A newer desired version resets the throttle immediately.
        #[test]
        fn newer_version_resets_throttle() {
            let mut throttle = SyncThrottle::default();
            assert!(throttle.should_attempt(2));
            assert!(!throttle.should_attempt(2));
            assert!(throttle.should_attempt(3));
        }
    }

    fn test_config(server_url: &str, version: i64) -> AgentConfig {
        AgentConfig {
            version,
            agent_id: Uuid::now_v7(),
            server: ServerConfig {
                url: server_url.to_string(),
                api_key: Some("sk_local".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Run the heartbeat loop until `done` returns true or `max_wait` passes.
    async fn run_until(
        config: AgentConfig,
        config_path: PathBuf,
        max_wait: Duration,
        done: impl Fn() -> bool,
    ) {
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let handle = tokio::spawn(run_heartbeat(
            Arc::new(RwLock::new(config)),
            Arc::new(RwLock::new(AgentStatus::new())),
            config_path,
            shutdown_rx,
        ));

        let deadline = tokio::time::Instant::now() + max_wait;
        while !done() && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        let _ = shutdown_tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
    }

    /// A heartbeat reporting a newer desired version makes the agent save the server config.
    #[tokio::test]
    async fn heartbeat_with_newer_desired_version_persists_server_config() {
        let mut server = mockito::Server::new_async().await;
        let local = test_config(&server.url(), 1);
        let envelope = serde_json::json!({
            "id": local.agent_id,
            "workspace_id": Uuid::now_v7(),
            "name": "from-server",
            "config_version_desired": 2,
            "base_config": {}
        });

        let heartbeat = server
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
            .with_body(envelope.to_string())
            .expect(1)
            .create_async()
            .await;
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        let check_path = path.clone();
        run_until(local, path, Duration::from_secs(5), move || {
            AgentConfig::from_file(&check_path).is_ok_and(|c| c.version == 2)
        })
        .await;

        let saved = AgentConfig::from_file(file.path()).unwrap();
        assert_eq!(saved.version, 2);
        assert_eq!(saved.agent_name, "from-server");
        assert_eq!(saved.server.api_key.as_deref(), Some("sk_local"));
        heartbeat.assert_async().await;
        configuration.assert_async().await;
    }

    /// A legacy 204 heartbeat never triggers a configuration fetch.
    #[tokio::test]
    async fn legacy_204_heartbeat_does_not_fetch_configuration() {
        let mut server = mockito::Server::new_async().await;
        let local = test_config(&server.url(), 1);
        let heartbeat = server
            .mock(
                "POST",
                format!("/agent/{}/heartbeat", local.agent_id).as_str(),
            )
            .with_status(204)
            .create_async()
            .await;
        let configuration = server
            .mock(
                "GET",
                format!("/agent/{}/configuration", local.agent_id).as_str(),
            )
            .expect(0)
            .create_async()
            .await;
        let file = NamedTempFile::new().unwrap();

        // The first heartbeat fires immediately; wait long enough for it to complete.
        run_until(
            local,
            file.path().to_path_buf(),
            Duration::from_millis(500),
            || false,
        )
        .await;

        heartbeat.assert_async().await;
        configuration.assert_async().await;
    }

    /// A desired version equal to the current one never triggers a configuration fetch.
    #[tokio::test]
    async fn heartbeat_with_current_version_does_not_fetch_configuration() {
        let mut server = mockito::Server::new_async().await;
        let local = test_config(&server.url(), 3);
        let heartbeat = server
            .mock(
                "POST",
                format!("/agent/{}/heartbeat", local.agent_id).as_str(),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"config_version_desired": 3}"#)
            .create_async()
            .await;
        let configuration = server
            .mock(
                "GET",
                format!("/agent/{}/configuration", local.agent_id).as_str(),
            )
            .expect(0)
            .create_async()
            .await;
        let file = NamedTempFile::new().unwrap();

        run_until(
            local,
            file.path().to_path_buf(),
            Duration::from_millis(500),
            || false,
        )
        .await;

        heartbeat.assert_async().await;
        configuration.assert_async().await;
    }
}
