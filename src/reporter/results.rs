//! Batch result reporter — drains the in-memory `ResultCache` and POSTs batches
//! to `POST /agent/{agentId}/results` on the central server.
//!
//! # Wire format
//!
//! The JSON payload matches `openapi::BatchMonitoringResults` (generated from
//! the OpenAPI spec). Since `openapi::MonitoringResult` and `openapi::CheckType`
//! are now the canonical types used end-to-end, results are serialized directly
//! without any adapter layer.
//!
//! # Peek-then-drain semantics
//!
//! 1. `peek_batch(n)` — clone the first N items without removing them.
//! 2. POST the batch.
//! 3. `drain_front(n)` on success only — guarantees at-least-once delivery.
//!    On failure the same items will be retried on the next tick.

use crate::agent_config::Config;
use crate::cache::ResultCache;
use crate::core::{AgentStatus, MonitoringResult};
use crate::error::{Error, Result};
use crate::openapi;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{debug, error, info};

// ============================================================
// Reporter loop
// ============================================================

/// Run the result-reporter loop.
///
/// Periodically drains a batch from `result_cache` and posts it to the server.
/// Items are only removed from the cache after the server acknowledges the POST
/// (peek-then-drain semantics). If the POST fails, the same items are retried
/// on the next tick.
///
/// The loop respects hot-reloaded configuration (server URL, API key, intervals,
/// cache settings) by reading a fresh config snapshot on every tick.
pub async fn run_result_reporter(
    config: Arc<RwLock<Config>>,
    result_cache: Arc<ResultCache>,
    agent_status: Arc<RwLock<AgentStatus>>,
    mut agent_shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting result reporter");

    if !config.read().storage.cache_enabled {
        info!("Result cache disabled in config, result reporter exiting");
        return Ok(());
    }

    let mut current_interval_duration =
        std::time::Duration::from_secs(config.read().storage.cache_report_interval_secs);
    let mut iv = interval(current_interval_duration);
    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = iv.tick() => {
                let config_snapshot = config.read().clone();

                // Exit early if caching is disabled via hot-reload.
                if !config_snapshot.storage.cache_enabled {
                    info!("Result cache disabled via hot-reload, result reporter exiting");
                    break;
                }

                // Hot-reload interval changes.
                let new_interval = std::time::Duration::from_secs(
                    config_snapshot.storage.cache_report_interval_secs,
                );
                if new_interval != current_interval_duration {
                    info!(
                        "Result report interval changed from {:?} to {:?}, recreating timer",
                        current_interval_duration, new_interval
                    );
                    current_interval_duration = new_interval;
                    iv = interval(current_interval_duration);
                    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                }

                if !config_snapshot.server.is_configured() {
                    debug!("Server not configured, skipping result batch send");
                    continue;
                }

                let batch_size = config_snapshot.storage.cache_batch_size;
                let batch = result_cache.peek_batch(batch_size).await;

                if batch.is_empty() {
                    debug!("Result cache empty, nothing to send");
                    continue;
                }

                debug!("Sending batch of {} results to server", batch.len());

                match send_result_batch(&config_snapshot, &batch).await {
                    Ok(()) => {
                        let sent = batch.len();
                        result_cache.drain_front(sent).await;
                        let remaining = result_cache.len().await;
                        agent_status.write().cached_reports = remaining as i64;
                        debug!(
                            "Sent {} results, {} remaining in cache",
                            sent, remaining
                        );
                    }
                    Err(e) => {
                        agent_status.write().failed_report_count += 1;
                        error!("Failed to send result batch: {}", e);
                    }
                }
            }
            _ = agent_shutdown_rx.recv() => {
                info!("Result reporter shutting down");
                break;
            }
        }
    }

    Ok(())
}

/// POST `POST /agent/{agentId}/results` with a batch payload.
async fn send_result_batch(config: &Config, batch: &[MonitoringResult]) -> Result<()> {
    let agent_id = config.agent_id;
    let server_url = &config.server.url;

    let client = reqwest::Client::builder()
        .timeout(config.server.timeout())
        .danger_accept_invalid_certs(!config.server.verify_tls)
        .build()?;

    let url = format!("{}/agent/{}/results", server_url, agent_id);
    let payload = openapi::BatchMonitoringResults {
        results: batch.to_vec(),
    };

    let mut request = client.post(&url).json(&payload);

    if let Some(api_key) = &config.server.api_key {
        request = request.header("X-Agent-API-Key", api_key);
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        return Err(Error::Network(format!(
            "Server returned {} for result batch POST",
            response.status()
        )));
    }

    // Parse and log the acknowledgment for observability.
    match response.json::<openapi::ResultsBatchAcknowledgment>().await {
        Ok(ack) => {
            debug!(
                "Server acknowledged batch: submission_id={}, accepted={}, duplicates_skipped={}",
                ack.submission_id,
                ack.accepted,
                ack.duplicates_skipped.unwrap_or(0),
            );
        }
        Err(e) => {
            // A parse failure is non-fatal — the server already returned 2xx.
            debug!("Could not parse ResultsBatchAcknowledgment body: {}", e);
        }
    }

    Ok(())
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        CheckType, Endpoint, MonitoringResult, PingCheck, PingCheckType, PingResult,
    };
    use chrono::Utc;
    use uuid::Uuid;

    fn make_result() -> MonitoringResult {
        MonitoringResult {
            id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            target: Endpoint::new("1.2.3.4"),
            check_type: CheckType::PingCheck(PingCheck {
                r#type: PingCheckType::Ping,
                result: PingResult {
                    resolved_ip: Some("1.2.3.4".to_string()),
                    successes: Some(3),
                    failures: Some(0),
                    success_latencies: Some(vec![1.0, 2.0, 3.0]),
                    avg_response_time_ms: Some(2.0),
                    errors: Some(vec![]),
                },
            }),
            timestamp: Utc::now(),
        }
    }

    mod serialisation_tests {
        use super::*;

        fn make_batch(results: Vec<MonitoringResult>) -> openapi::BatchMonitoringResults {
            openapi::BatchMonitoringResults { results }
        }

        #[test]
        fn ping_serialises_with_type_result_envelope() {
            let result = make_result();
            let batch = make_batch(vec![result]);
            let json = serde_json::to_value(&batch).expect("serialisation should not fail");
            let check = &json["results"][0]["check_type"];
            assert_eq!(
                check["type"].as_str(),
                Some("ping"),
                "discriminator 'type' must be 'ping'"
            );
            assert!(
                check["result"].is_object(),
                "'result' envelope must be present"
            );
            assert_eq!(check["result"]["successes"].as_i64(), Some(3));
        }

        #[test]
        fn tcpconnect_serialises_correct_discriminator() {
            use crate::core::{TcpConnectCheck, TcpConnectCheckType, TcpConnectResult};
            let mut result = make_result();
            result.check_type = CheckType::TcpConnectCheck(TcpConnectCheck {
                r#type: TcpConnectCheckType::Tcpconnect,
                result: TcpConnectResult {
                    connected: Some(true),
                    connect_time_ms: Some(5.0),
                    error: None,
                    resolved_ip: None,
                },
            });
            let batch = make_batch(vec![result]);
            let json = serde_json::to_value(&batch).expect("serialisation should not fail");
            assert_eq!(
                json["results"][0]["check_type"]["type"].as_str(),
                Some("tcpconnect")
            );
        }

        #[test]
        fn httpget_serialises_correct_discriminator() {
            use crate::core::{HttpGetCheck, HttpGetCheckType, HttpGetResult};
            let mut result = make_result();
            result.check_type = CheckType::HttpGetCheck(HttpGetCheck {
                r#type: HttpGetCheckType::Httpget,
                result: HttpGetResult {
                    status_code: Some(200),
                    response_time_ms: Some(100.0),
                    response_size_bytes: Some(1024),
                    error: None,
                    success: Some(true),
                },
            });
            let batch = make_batch(vec![result]);
            let json = serde_json::to_value(&batch).expect("serialisation should not fail");
            assert_eq!(
                json["results"][0]["check_type"]["type"].as_str(),
                Some("httpget")
            );
        }

        #[test]
        fn empty_batch_serialises_as_empty_results_array() {
            let batch = make_batch(vec![]);
            let json = serde_json::to_value(&batch).expect("serialisation should not fail");
            assert_eq!(json["results"].as_array().map(|a| a.len()), Some(0));
        }

        #[test]
        fn multiple_results_all_present_in_json() {
            let r1 = make_result();
            let r2 = make_result();
            let batch = make_batch(vec![r1, r2]);
            let json = serde_json::to_value(&batch).expect("serialisation should not fail");
            assert_eq!(json["results"].as_array().map(|a| a.len()), Some(2));
            assert_eq!(
                json["results"][0]["check_type"]["type"].as_str(),
                Some("ping")
            );
            assert_eq!(
                json["results"][1]["check_type"]["type"].as_str(),
                Some("ping")
            );
        }
    }
}

// ============================================================
// Reporter loop behaviour tests (with mock HTTP server)
// ============================================================

#[cfg(test)]
mod reporter_loop_tests {
    use super::run_result_reporter;
    use crate::agent_config::{Config, MonitoringConfig, ServerConfig, StorageConfig};
    use crate::cache::ResultCache;
    use crate::core::{
        AgentStatus, CheckType, Endpoint, MonitoringResult, PingCheck, PingCheckType, PingResult,
    };
    use parking_lot::RwLock;
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;

    fn make_ping_result(address: &str) -> MonitoringResult {
        MonitoringResult {
            id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            target: Endpoint::new(address),
            check_type: CheckType::PingCheck(PingCheck {
                r#type: PingCheckType::Ping,
                result: PingResult {
                    resolved_ip: Some(address.to_string()),
                    successes: Some(3),
                    failures: Some(0),
                    success_latencies: Some(vec![1.0, 2.0, 3.0]),
                    avg_response_time_ms: Some(2.0),
                    errors: Some(vec![]),
                },
            }),
            timestamp: chrono::Utc::now(),
        }
    }

    fn make_cache(max_size: usize, max_age_secs: u64) -> Arc<ResultCache> {
        Arc::new(ResultCache::new(
            max_size,
            Duration::from_secs(max_age_secs),
        ))
    }

    fn make_config(server_url: &str) -> Arc<RwLock<Config>> {
        let storage = StorageConfig {
            cache_enabled: true,
            cache_batch_size: 10,
            cache_report_interval_secs: 1,
            ..StorageConfig::default()
        };
        let server = ServerConfig {
            url: server_url.to_string(),
            api_key: Some("test-api-key".to_string()),
            ..ServerConfig::default()
        };
        Arc::new(RwLock::new(Config {
            version: 1,
            agent_id: Uuid::new_v4(),
            agent_name: "Test Agent".to_string(),
            tags: vec![],
            monitoring: MonitoringConfig::default(),
            server,
            storage,
            endpoints: vec![],
        }))
    }

    /// Spawn a minimal HTTP server that accepts one POST and returns 202.
    /// Returns the bound address and a oneshot receiver that fires once the
    /// server receives a request.
    async fn spawn_mock_server_202() -> (
        std::net::SocketAddr,
        tokio::sync::oneshot::Receiver<Vec<u8>>,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let ack_body = r#"{"submission_id":"00000000-0000-0000-0000-000000000001","accepted":1,"received_at":"2026-01-01T00:00:00Z"}"#;
        let response = format!(
            "HTTP/1.1 202 Accepted\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            ack_body.len(),
            ack_body,
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = vec![0u8; 16384];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                buf.truncate(n);
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = tx.send(buf);
            }
        });

        (addr, rx)
    }

    /// Spawn a mock server that always returns 503 Service Unavailable.
    async fn spawn_mock_server_503() -> std::net::SocketAddr {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let mut buf = vec![0u8; 4096];
                    let _ = stream.read(&mut buf).await;
                    let _ = stream
                        .write_all(
                            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n",
                        )
                        .await;
                }
            }
        });

        addr
    }

    #[tokio::test]
    async fn reporter_drains_cache_on_successful_send() {
        let (addr, body_rx) = spawn_mock_server_202().await;
        let server_url = format!("http://{}", addr);

        let cache = make_cache(100, 3600);
        let config = make_config(&server_url);
        config.write().storage.cache_report_interval_secs = 1;

        for i in 0..5 {
            cache.push(make_ping_result(&format!("10.0.0.{}", i))).await;
        }
        assert_eq!(cache.len().await, 5);

        let agent_status = Arc::new(RwLock::new(AgentStatus::default()));
        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

        let reporter_task = tokio::spawn({
            let cache = Arc::clone(&cache);
            let config = Arc::clone(&config);
            let status = Arc::clone(&agent_status);
            async move { run_result_reporter(config, cache, status, shutdown_rx).await }
        });

        let body = tokio::time::timeout(Duration::from_secs(5), body_rx)
            .await
            .expect("timeout waiting for POST request")
            .expect("server channel closed");

        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = shutdown_tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(2), reporter_task).await;

        assert_eq!(
            cache.len().await,
            0,
            "cache must be fully drained after successful server acknowledgment"
        );
        let body_str = String::from_utf8_lossy(&body);
        assert!(
            body_str.contains("results"),
            "POST body should contain 'results' key"
        );
    }

    #[tokio::test]
    async fn reporter_does_not_drain_cache_on_server_error() {
        let addr = spawn_mock_server_503().await;
        let server_url = format!("http://{}", addr);

        let cache = make_cache(100, 3600);
        let config = make_config(&server_url);
        config.write().storage.cache_report_interval_secs = 1;

        for i in 0..3 {
            cache.push(make_ping_result(&format!("10.0.0.{}", i))).await;
        }
        assert_eq!(cache.len().await, 3);

        let agent_status = Arc::new(RwLock::new(AgentStatus::default()));
        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

        let reporter_task = tokio::spawn({
            let cache = Arc::clone(&cache);
            let config = Arc::clone(&config);
            let status = Arc::clone(&agent_status);
            async move { run_result_reporter(config, cache, status, shutdown_rx).await }
        });

        tokio::time::sleep(Duration::from_millis(1500)).await;
        let _ = shutdown_tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(2), reporter_task).await;

        assert_eq!(
            cache.len().await,
            3,
            "cache must not be drained when server returns a non-2xx response"
        );
        assert!(
            agent_status.read().failed_report_count > 0,
            "failed_report_count must be incremented on server error"
        );
    }

    #[tokio::test]
    async fn reporter_exits_immediately_when_cache_disabled() {
        let cache = make_cache(100, 3600);
        let config = Arc::new(RwLock::new(Config {
            version: 1,
            agent_id: Uuid::new_v4(),
            agent_name: "Test".to_string(),
            tags: vec![],
            monitoring: MonitoringConfig::default(),
            server: ServerConfig {
                url: "http://127.0.0.1:1".to_string(),
                ..ServerConfig::default()
            },
            storage: StorageConfig {
                cache_enabled: false,
                ..StorageConfig::default()
            },
            endpoints: vec![],
        }));

        cache.push(make_ping_result("1.1.1.1")).await;
        assert_eq!(cache.len().await, 1);

        let agent_status = Arc::new(RwLock::new(AgentStatus::default()));
        let (_shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

        let result = tokio::time::timeout(
            Duration::from_secs(2),
            run_result_reporter(config, Arc::clone(&cache), agent_status, shutdown_rx),
        )
        .await
        .expect("reporter should exit quickly when cache is disabled");

        assert!(result.is_ok());
        assert_eq!(
            cache.len().await,
            1,
            "cache must be untouched when reporting is disabled"
        );
    }

    #[tokio::test]
    async fn reporter_noop_on_empty_cache() {
        let (addr, _body_rx) = spawn_mock_server_202().await;
        let server_url = format!("http://{}", addr);

        let cache = make_cache(100, 3600); // empty
        let config = make_config(&server_url);
        config.write().storage.cache_report_interval_secs = 1;

        let agent_status = Arc::new(RwLock::new(AgentStatus::default()));
        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

        let reporter_task = tokio::spawn({
            let cache = Arc::clone(&cache);
            let config = Arc::clone(&config);
            let status = Arc::clone(&agent_status);
            async move { run_result_reporter(config, cache, status, shutdown_rx).await }
        });

        tokio::time::sleep(Duration::from_millis(1200)).await;
        let _ = shutdown_tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(2), reporter_task).await;

        assert_eq!(cache.len().await, 0);
        assert_eq!(
            agent_status.read().failed_report_count,
            0,
            "no failures should be recorded when cache is empty"
        );
    }
}
