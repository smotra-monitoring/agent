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
//!
//! # Rejected batches
//!
//! HTTP 400/422 means the server will never accept the payload, so retrying
//! would block the queue forever. Such a batch is dropped without retry, counted
//! in `dropped_result_count` and appended to `{cache_dir}/rejected_results.jsonl`
//! (rotated to `.1` at 5 MiB). Every other failure (network, 401/403/404, 5xx, ...)
//! keeps the batch for retry.

use crate::cache::ResultCache;
use crate::core::{AgentStatus, CheckType, MonitoringResult};
use crate::error::{Error, Result};
use crate::http_client::build_client;
use crate::openapi::{self, AgentConfig};
use parking_lot::RwLock;
use reqwest::StatusCode;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

const REJECTED_FILE_NAME: &str = "rejected_results.jsonl";
const REJECTED_FILE_MAX_BYTES: u64 = 5 * 1024 * 1024;
const MAX_LOGGED_BODY_CHARS: usize = 1024;

/// Result of a single batch POST attempt.
enum SendOutcome {
    Accepted,
    /// Transient or environmental failure; the batch must be kept.
    Retryable(Error),
    /// The server permanently refused the payload; the batch must be dropped.
    Rejected {
        status: StatusCode,
        body: String,
    },
}

/// Only 400 and 422 are permanent; 404 stays retryable because an operator can restore the agent.
fn is_rejection(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY
    )
}

fn check_type_name(check_type: &CheckType) -> &'static str {
    match check_type {
        CheckType::PingCheck(_) => "ping",
        CheckType::TracerouteCheck(_) => "traceroute",
        CheckType::TcpConnectCheck(_) => "tcpconnect",
        CheckType::UdpConnectCheck(_) => "udpconnect",
        CheckType::HttpGetCheck(_) => "httpget",
        CheckType::PluginCheck(_) => "plugin",
    }
}

fn truncate_body(body: &str) -> String {
    match body.char_indices().nth(MAX_LOGGED_BODY_CHARS) {
        Some((idx, _)) => format!("{}...[truncated]", &body[..idx]),
        None => body.to_string(),
    }
}

/// Append rejected results to `{dir}/rejected_results.jsonl`, rotating to `.1` once `max_bytes` is reached.
async fn save_rejected_batch(
    dir: &Path,
    max_bytes: u64,
    status: StatusCode,
    body: &str,
    batch: &[MonitoringResult],
) -> std::io::Result<()> {
    tokio::fs::create_dir_all(dir).await?;
    let path = dir.join(REJECTED_FILE_NAME);

    if let Ok(meta) = tokio::fs::metadata(&path).await {
        if meta.len() >= max_bytes {
            tokio::fs::rename(&path, dir.join(format!("{REJECTED_FILE_NAME}.1"))).await?;
        }
    }

    let rejected_at = chrono::Utc::now();
    let body = truncate_body(body);
    let mut lines = String::new();
    for result in batch {
        let record = serde_json::json!({
            "rejected_at": rejected_at,
            "status": status.as_u16(),
            "server_response": body,
            "result": result,
        });
        lines.push_str(&record.to_string());
        lines.push('\n');
    }

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await?;
    file.write_all(lines.as_bytes()).await?;
    file.flush().await
}

/// Remove the first `count` items from the cache and refresh the cache stats in the agent status.
async fn drain_and_refresh_stats(
    result_cache: &ResultCache,
    agent_status: &RwLock<AgentStatus>,
    count: usize,
) -> usize {
    result_cache.drain_front(count).await;
    let stats = result_cache.stats().await;
    let mut s = agent_status.write();
    s.cache_stats.len = stats.len as i64;
    s.cache_stats.capacity = stats.capacity as i64;
    stats.len
}

// ============================================================
// Reporter loop
// ============================================================

/// Run the result-reporter loop.
///
/// Periodically drains a batch from `result_cache` and posts it to the server.
/// Items are only removed from the cache after the server acknowledges the POST
/// (peek-then-drain semantics). If the POST fails, the same items are retried
/// on the next tick, except for batches the server rejects with 400/422, which
/// are dropped (see the module docs).
///
/// The loop respects hot-reloaded configuration (server URL, API key, intervals,
/// cache settings) by reading a fresh config snapshot on every tick.
pub async fn run_result_reporter(
    config: Arc<RwLock<AgentConfig>>,
    result_cache: Arc<ResultCache>,
    agent_status: Arc<RwLock<AgentStatus>>,
    mut agent_shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting result reporter");

    if !config.read().storage.cache_enabled {
        info!("Result cache disabled in config, result reporter exiting");
        return Ok(());
    }

    let mut current_interval_duration = config.read().storage.cache_report_interval();
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
                let new_interval = config_snapshot.storage.cache_report_interval();
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

                let batch_size = config_snapshot.storage.cache_batch_size as usize;
                let batch = result_cache.peek_batch(batch_size).await;

                if batch.is_empty() {
                    debug!("Result cache empty, nothing to send");
                    continue;
                }

                debug!("Sending batch of {} results to server", batch.len());

                match send_result_batch(&config_snapshot, &batch).await {
                    SendOutcome::Accepted => {
                        let sent = batch.len();
                        let remaining =
                            drain_and_refresh_stats(&result_cache, &agent_status, sent).await;
                        debug!(
                            "Sent {} results, {} remaining in cache",
                            sent, remaining
                        );
                    }
                    SendOutcome::Rejected { status, body } => {
                        let ids: Vec<String> = batch.iter().map(|r| r.id.to_string()).collect();
                        let check_types: BTreeSet<&str> =
                            batch.iter().map(|r| check_type_name(&r.check_type)).collect();
                        warn!(
                            "Server rejected result batch with {}, dropping {} results: ids=[{}] check_types={:?} body={}",
                            status,
                            batch.len(),
                            ids.join(","),
                            check_types,
                            truncate_body(&body),
                        );

                        let dir = Path::new(&config_snapshot.storage.cache_dir);
                        if let Err(e) = save_rejected_batch(
                            dir,
                            REJECTED_FILE_MAX_BYTES,
                            status,
                            &body,
                            &batch,
                        )
                        .await
                        {
                            warn!("Could not save rejected results in {}: {}", dir.display(), e);
                        }

                        drain_and_refresh_stats(&result_cache, &agent_status, batch.len()).await;
                        agent_status.write().dropped_result_count += batch.len() as i64;
                    }
                    SendOutcome::Retryable(e) => {
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
async fn send_result_batch(config: &AgentConfig, batch: &[MonitoringResult]) -> SendOutcome {
    let agent_id = config.agent_id;
    let server_url = &config.server.url;

    let client = match build_client(config.server.timeout(), config.server.verify_tls) {
        Ok(client) => client,
        Err(e) => return SendOutcome::Retryable(e.into()),
    };

    let url = format!("{}/agent/{}/results", server_url, agent_id);
    let payload = openapi::BatchMonitoringResults {
        results: batch.to_vec(),
    };

    let mut request = client.post(&url).json(&payload);

    if let Some(api_key) = &config.server.api_key {
        request = request.header("X-Agent-API-Key", api_key);
    }

    let response = match request.send().await {
        Ok(response) => response,
        Err(e) => return SendOutcome::Retryable(e.into()),
    };

    let status = response.status();
    if is_rejection(status) {
        let body = response.text().await.unwrap_or_default();
        return SendOutcome::Rejected { status, body };
    }
    if !status.is_success() {
        return SendOutcome::Retryable(Error::Network(format!(
            "Server returned {} for result batch POST",
            status
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

    SendOutcome::Accepted
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{CheckType, MonitoringResult, PingCheck, PingCheckType, PingResult};
    use chrono::Utc;
    use uuid::Uuid;

    fn make_result() -> MonitoringResult {
        MonitoringResult {
            id: Uuid::now_v7(),
            agent_id: Uuid::now_v7(),
            endpoint_id: Uuid::now_v7(),
            check_type: CheckType::PingCheck(PingCheck {
                r#type: PingCheckType::Ping,
                result: PingResult {
                    resolved_ip: "1.2.3.4".to_string(),
                    successes: 3,
                    failures: 0,
                    success_latencies: vec![1.0, 2.0, 3.0],
                    error_details: None,
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
                    connected: true,
                    connect_time_ms: Some(5.0),
                    error_details: None,
                    resolved_ip: String::new(),
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
                    status_code: 200,
                    response_time_ms: Some(100.0),
                    response_size_bytes: Some(1024),
                    error_details: None,
                    success: true,
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

    mod is_rejection_tests {
        use super::*;

        /// Only 400/422 are permanent; everything else must keep the batch.
        #[test]
        fn only_400_and_422_are_rejections() {
            for code in [400u16, 422] {
                assert!(is_rejection(StatusCode::from_u16(code).unwrap()), "{code}");
            }
            for code in [200u16, 202, 401, 403, 404, 408, 413, 429, 500, 502, 503] {
                assert!(!is_rejection(StatusCode::from_u16(code).unwrap()), "{code}");
            }
        }
    }

    mod truncate_body_tests {
        use super::*;

        #[test]
        fn short_body_is_unchanged() {
            assert_eq!(truncate_body("bad request"), "bad request");
        }

        #[test]
        fn empty_body_is_unchanged() {
            assert_eq!(truncate_body(""), "");
        }

        /// Multi-byte characters must not be split mid-codepoint.
        #[test]
        fn long_multibyte_body_is_truncated_on_char_boundary() {
            let body = "é".repeat(MAX_LOGGED_BODY_CHARS + 10);
            let out = truncate_body(&body);
            assert!(out.ends_with("...[truncated]"));
            assert_eq!(
                out.chars().count(),
                MAX_LOGGED_BODY_CHARS + "...[truncated]".chars().count()
            );
        }
    }

    mod save_rejected_batch_tests {
        use super::*;

        #[tokio::test]
        async fn writes_one_json_line_per_result() {
            let dir = tempfile::tempdir().unwrap();
            let batch = vec![make_result(), make_result()];

            save_rejected_batch(
                dir.path(),
                REJECTED_FILE_MAX_BYTES,
                StatusCode::UNPROCESSABLE_ENTITY,
                "unknown check type",
                &batch,
            )
            .await
            .unwrap();

            let content = std::fs::read_to_string(dir.path().join(REJECTED_FILE_NAME)).unwrap();
            let lines: Vec<&str> = content.lines().collect();
            assert_eq!(lines.len(), 2);
            let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
            assert_eq!(first["status"], 422);
            assert_eq!(first["server_response"], "unknown check type");
            assert_eq!(first["result"]["id"], batch[0].id.to_string());
        }

        #[tokio::test]
        async fn creates_missing_directory() {
            let dir = tempfile::tempdir().unwrap();
            let nested = dir.path().join("a").join("b");

            save_rejected_batch(
                &nested,
                REJECTED_FILE_MAX_BYTES,
                StatusCode::BAD_REQUEST,
                "",
                &[make_result()],
            )
            .await
            .unwrap();

            assert!(nested.join(REJECTED_FILE_NAME).exists());
        }

        #[tokio::test]
        async fn rotates_when_size_cap_reached_and_keeps_single_backup() {
            let dir = tempfile::tempdir().unwrap();
            let save = |batch: Vec<MonitoringResult>| {
                let path = dir.path().to_path_buf();
                async move {
                    save_rejected_batch(&path, 1, StatusCode::BAD_REQUEST, "x", &batch)
                        .await
                        .unwrap();
                }
            };

            save(vec![make_result()]).await;
            save(vec![make_result()]).await;
            save(vec![make_result(), make_result()]).await;

            let current = std::fs::read_to_string(dir.path().join(REJECTED_FILE_NAME)).unwrap();
            let backup =
                std::fs::read_to_string(dir.path().join(format!("{REJECTED_FILE_NAME}.1")))
                    .unwrap();
            assert_eq!(
                current.lines().count(),
                2,
                "newest batch only in current file"
            );
            assert_eq!(backup.lines().count(), 1, "previous backup is overwritten");
        }

        #[tokio::test]
        async fn fails_when_directory_is_a_file() {
            let dir = tempfile::tempdir().unwrap();
            let not_a_dir = dir.path().join("file");
            std::fs::write(&not_a_dir, b"").unwrap();

            let result = save_rejected_batch(
                &not_a_dir,
                REJECTED_FILE_MAX_BYTES,
                StatusCode::BAD_REQUEST,
                "",
                &[make_result()],
            )
            .await;

            assert!(result.is_err());
        }
    }
}

// ============================================================
// Reporter loop behaviour tests (with mock HTTP server)
// ============================================================

#[cfg(test)]
mod reporter_loop_tests {
    use super::run_result_reporter;
    use crate::cache::ResultCache;
    use crate::core::{
        AgentStatus, CheckType, MonitoringResult, PingCheck, PingCheckType, PingResult,
    };
    use crate::openapi::{AgentConfig, MonitoringConfig, ServerConfig, StorageConfig};
    use parking_lot::RwLock;
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;

    fn make_ping_result(address: &str) -> MonitoringResult {
        MonitoringResult {
            id: Uuid::now_v7(),
            agent_id: Uuid::now_v7(),
            endpoint_id: Uuid::now_v7(),
            check_type: CheckType::PingCheck(PingCheck {
                r#type: PingCheckType::Ping,
                result: PingResult {
                    resolved_ip: address.to_string(),
                    successes: 3,
                    failures: 0,
                    success_latencies: vec![1.0, 2.0, 3.0],
                    error_details: None,
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

    fn make_config(server_url: &str) -> Arc<RwLock<AgentConfig>> {
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
        Arc::new(RwLock::new(AgentConfig {
            version: 1,
            agent_id: Uuid::now_v7(),
            agent_name: "Test Agent".to_string(),
            tags: vec![],
            monitoring: MonitoringConfig::default(),
            server,
            storage,
            self_upgrade: Default::default(),
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
                        .write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n")
                        .await;
                }
            }
        });

        addr
    }

    /// Spawn a mock server that always answers with the given status line and body.
    async fn spawn_mock_server_status(
        status_line: &'static str,
        body: &'static str,
    ) -> std::net::SocketAddr {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let response = format!(
            "HTTP/1.1 {}\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
            status_line,
            body.len(),
            body
        );

        tokio::spawn(async move {
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let mut buf = vec![0u8; 16384];
                    let _ = stream.read(&mut buf).await;
                    let _ = stream.write_all(response.as_bytes()).await;
                }
            }
        });

        addr
    }

    /// Run the reporter against `status_line` for ~1.5s with a 3-result cache.
    async fn run_with_status(
        status_line: &'static str,
        body: &'static str,
        cache_dir: &std::path::Path,
    ) -> (Arc<ResultCache>, Arc<RwLock<AgentStatus>>) {
        let addr = spawn_mock_server_status(status_line, body).await;
        let cache = make_cache(100, 3600);
        let config = make_config(&format!("http://{}", addr));
        config.write().storage.cache_dir = cache_dir.to_string_lossy().into_owned();

        for i in 0..3 {
            cache.push(make_ping_result(&format!("10.0.0.{}", i))).await;
        }

        let agent_status = Arc::new(RwLock::new(AgentStatus::default()));
        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
        let reporter_task = tokio::spawn({
            let cache = Arc::clone(&cache);
            let status = Arc::clone(&agent_status);
            async move { run_result_reporter(config, cache, status, shutdown_rx).await }
        });

        tokio::time::sleep(Duration::from_millis(1500)).await;
        let _ = shutdown_tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(2), reporter_task).await;
        (cache, agent_status)
    }

    /// Regression: a permanently rejected batch used to stay at the head of the queue forever.
    #[tokio::test]
    async fn reporter_drops_and_records_batch_rejected_with_422() {
        let dir = tempfile::tempdir().unwrap();
        let (cache, status) =
            run_with_status("422 Unprocessable Entity", "unknown check type", dir.path()).await;

        assert_eq!(cache.len().await, 0, "rejected batch must be drained");
        assert_eq!(status.read().dropped_result_count, 3);
        assert_eq!(
            status.read().failed_report_count,
            0,
            "a rejection is not retried, so it is not a failed report"
        );
        assert_eq!(status.read().cache_stats.len, 0);

        let content = std::fs::read_to_string(dir.path().join("rejected_results.jsonl")).unwrap();
        assert_eq!(content.lines().count(), 3);
        assert!(content.contains("unknown check type"));
    }

    #[tokio::test]
    async fn reporter_drops_batch_rejected_with_400() {
        let dir = tempfile::tempdir().unwrap();
        let (cache, status) = run_with_status("400 Bad Request", "bad json", dir.path()).await;

        assert_eq!(cache.len().await, 0);
        assert_eq!(status.read().dropped_result_count, 3);
    }

    /// A rejected batch must still be dropped when the rejected file cannot be written.
    #[tokio::test]
    async fn reporter_drops_rejected_batch_even_if_file_cannot_be_written() {
        let dir = tempfile::tempdir().unwrap();
        let not_a_dir = dir.path().join("file");
        std::fs::write(&not_a_dir, b"").unwrap();

        let (cache, status) = run_with_status("422 Unprocessable Entity", "x", &not_a_dir).await;

        assert_eq!(cache.len().await, 0);
        assert_eq!(status.read().dropped_result_count, 3);
    }

    /// 401/403/404/429/5xx are not data problems; the batch must be kept and nothing recorded.
    async fn assert_batch_kept(status_line: &'static str) {
        let dir = tempfile::tempdir().unwrap();
        let (cache, status) = run_with_status(status_line, "nope", dir.path()).await;

        assert_eq!(cache.len().await, 3, "{status_line}: batch must be kept");
        assert_eq!(status.read().dropped_result_count, 0, "{status_line}");
        assert!(status.read().failed_report_count > 0, "{status_line}");
        assert!(
            !dir.path().join("rejected_results.jsonl").exists(),
            "{status_line}: nothing must be written to the rejected file"
        );
    }

    #[tokio::test]
    async fn reporter_keeps_batch_on_401() {
        assert_batch_kept("401 Unauthorized").await;
    }

    #[tokio::test]
    async fn reporter_keeps_batch_on_403() {
        assert_batch_kept("403 Forbidden").await;
    }

    #[tokio::test]
    async fn reporter_keeps_batch_on_404() {
        assert_batch_kept("404 Not Found").await;
    }

    #[tokio::test]
    async fn reporter_keeps_batch_on_429() {
        assert_batch_kept("429 Too Many Requests").await;
    }

    #[tokio::test]
    async fn reporter_keeps_batch_on_500() {
        assert_batch_kept("500 Internal Server Error").await;
    }

    #[tokio::test]
    async fn reporter_keeps_batch_on_network_error() {
        // Nothing listens on port 1, so the connection is refused.
        let dir = tempfile::tempdir().unwrap();
        let cache = make_cache(100, 3600);
        let config = make_config("http://127.0.0.1:1");
        config.write().storage.cache_dir = dir.path().to_string_lossy().into_owned();
        cache.push(make_ping_result("10.0.0.1")).await;

        let agent_status = Arc::new(RwLock::new(AgentStatus::default()));
        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
        let task = tokio::spawn({
            let cache = Arc::clone(&cache);
            let status = Arc::clone(&agent_status);
            async move { run_result_reporter(config, cache, status, shutdown_rx).await }
        });
        tokio::time::sleep(Duration::from_millis(1200)).await;
        let _ = shutdown_tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(2), task).await;

        assert_eq!(cache.len().await, 1);
        assert_eq!(agent_status.read().dropped_result_count, 0);
        assert!(agent_status.read().failed_report_count > 0);
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

        let req_body = tokio::time::timeout(Duration::from_secs(5), body_rx)
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
        let body_str = String::from_utf8_lossy(&req_body);
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
        let config = Arc::new(RwLock::new(AgentConfig {
            version: 1,
            agent_id: Uuid::now_v7(),
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
            self_upgrade: Default::default(),
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
