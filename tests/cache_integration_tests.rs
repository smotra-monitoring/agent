//! Integration tests for the in-memory result cache and result reporter.
//!
//! These tests exercise the full pipeline:
//!   push results into cache → peek_batch / drain_front lifecycle
//!   → run_result_reporter loop with a mock HTTP server
//!
//! The mock server is implemented with `tokio` + `std::net::TcpListener` so we
//! don't need an extra test dependency.

use parking_lot::RwLock;
use smotra::{
    CacheStats, Config, MonitoringConfig, ResultCache, ServerConfig, StorageConfig,
};
use smotra::{CheckType, Endpoint, MonitoringResult, PingCheck, PingCheckType, PingResult};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

// ============================================================
// Helpers
// ============================================================

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
        agent_name: "Integration Test Agent".to_string(),
        tags: vec![],
        monitoring: MonitoringConfig::default(),
        server,
        storage,
        endpoints: vec![],
    }))
}

// ============================================================
// Cache lifecycle tests (no network)
// ============================================================

#[tokio::test]
async fn cache_push_peek_drain_roundtrip() {
    let cache = make_cache(100, 3600);

    // Push 5 results
    for i in 0..5 {
        cache.push(make_ping_result(&format!("10.0.0.{}", i))).await;
    }
    assert_eq!(cache.len().await, 5);

    // Peek 3 — cache unchanged
    let batch = cache.peek_batch(3).await;
    assert_eq!(batch.len(), 3);
    assert_eq!(cache.len().await, 5, "peek must not remove entries");

    // Drain 3 — simulate confirmed server send
    cache.drain_front(3).await;
    assert_eq!(cache.len().await, 2);

    // Remaining entries are the last 2
    let remaining = cache.peek_batch(10).await;
    assert_eq!(remaining[0].target.address, "10.0.0.3");
    assert_eq!(remaining[1].target.address, "10.0.0.4");
}

#[tokio::test]
async fn cache_ttl_eviction_integration() {
    // Use a 50ms TTL so we can trigger expiry in a test
    let cache = Arc::new(ResultCache::new(100, Duration::from_millis(50)));

    cache.push(make_ping_result("1.1.1.1")).await;
    cache.push(make_ping_result("2.2.2.2")).await;
    assert_eq!(cache.len().await, 2);

    // Wait for entries to become stale
    tokio::time::sleep(Duration::from_millis(100)).await;

    // A push triggers lazy TTL eviction
    cache.push(make_ping_result("3.3.3.3")).await;
    let remaining = cache.peek_batch(10).await;
    assert_eq!(remaining.len(), 1, "stale entries should be evicted");
    assert_eq!(remaining[0].target.address, "3.3.3.3");
}

#[tokio::test]
async fn cache_size_cap_evicts_oldest() {
    let cache = make_cache(3, 3600);
    for i in 0..5usize {
        let r = make_ping_result(&format!("10.0.0.{}", i));
        cache.push(r).await;
    }

    assert_eq!(cache.len().await, 3, "cache should be capped at 3");
    let batch = cache.peek_batch(3).await;
    // The 3 most recently pushed (indices 2, 3, 4)
    assert_eq!(batch[0].target.address, "10.0.0.2");
    assert_eq!(batch[1].target.address, "10.0.0.3");
    assert_eq!(batch[2].target.address, "10.0.0.4");
}

#[tokio::test]
async fn cache_stats_reflect_state() {
    let cache = make_cache(50, 3600);
    assert_eq!(cache.stats().await, CacheStats { len: 0, capacity: 50 });

    cache.push(make_ping_result("1.1.1.1")).await;
    assert_eq!(cache.stats().await, CacheStats { len: 1, capacity: 50 });

    cache.drain_front(1).await;
    assert_eq!(cache.stats().await, CacheStats { len: 0, capacity: 50 });
}

// ============================================================
// run_result_reporter integration tests (with mock HTTP server)
// ============================================================

/// Start a minimal HTTP server that accepts one POST and returns 202.
/// Returns the bound address and a oneshot receiver that fires once the server
/// receives a valid request.
async fn spawn_mock_server_202(
    _expected_path_prefix: &'static str,
) -> (std::net::SocketAddr, tokio::sync::oneshot::Receiver<Vec<u8>>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    // Minimal valid ResultsBatchAcknowledgment body.
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

/// Start a mock server that always returns 503 Service Unavailable.
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

#[tokio::test]
async fn result_reporter_drains_cache_on_success() {
    let (addr, body_rx) = spawn_mock_server_202("/agent/").await;
    let server_url = format!("http://{}", addr);

    let cache = make_cache(100, 3600);
    let config = make_config(&server_url);
    // Set a very short report interval so the test doesn't have to wait long.
    config.write().storage.cache_report_interval_secs = 1;

    // Pre-populate the cache with 5 results
    for i in 0..5 {
        cache.push(make_ping_result(&format!("10.0.0.{}", i))).await;
    }
    assert_eq!(cache.len().await, 5);

    let agent_status = Arc::new(RwLock::new(smotra::AgentStatus::default()));
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
    let cache_clone = Arc::clone(&cache);
    let config_clone = Arc::clone(&config);
    let status_clone = Arc::clone(&agent_status);

    let reporter_task = tokio::spawn(async move {
        smotra::run_result_reporter(config_clone, cache_clone, status_clone, shutdown_rx).await
    });

    // Wait for the HTTP request to arrive (server received request)
    let body = tokio::time::timeout(Duration::from_secs(5), body_rx)
        .await
        .expect("timeout waiting for POST request")
        .expect("server channel closed");

    // Give the reporter a tick to drain
    tokio::time::sleep(Duration::from_millis(100)).await;

    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(2), reporter_task).await;

    // Cache should be empty after successful send
    assert_eq!(
        cache.len().await,
        0,
        "cache must be drained after successful server acknowledgment"
    );

    // Verify the HTTP body contained our results
    let body_str = String::from_utf8_lossy(&body);
    assert!(
        body_str.contains("results"),
        "POST body should contain 'results' key"
    );
}

#[tokio::test]
async fn result_reporter_does_not_drain_on_server_failure() {
    let addr = spawn_mock_server_503().await;
    let server_url = format!("http://{}", addr);

    let cache = make_cache(100, 3600);
    let config = make_config(&server_url);
    config.write().storage.cache_report_interval_secs = 1;

    // Pre-populate cache
    for i in 0..3 {
        cache.push(make_ping_result(&format!("10.0.0.{}", i))).await;
    }
    let initial_len = cache.len().await;
    assert_eq!(initial_len, 3);

    let agent_status = Arc::new(RwLock::new(smotra::AgentStatus::default()));
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
    let cache_clone = Arc::clone(&cache);
    let config_clone = Arc::clone(&config);
    let status_clone = Arc::clone(&agent_status);

    let reporter_task = tokio::spawn(async move {
        smotra::run_result_reporter(config_clone, cache_clone, status_clone, shutdown_rx).await
    });

    // Let one tick elapse
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(2), reporter_task).await;

    // Cache must NOT have been drained on 503
    assert_eq!(
        cache.len().await,
        3,
        "cache must not be drained when server returns an error"
    );

    // failed_report_count should be incremented
    assert!(
        agent_status.read().failed_report_count > 0,
        "failed_report_count must be incremented on server error"
    );
}

#[tokio::test]
async fn result_reporter_skips_when_cache_disabled() {
    let (addr, _body_rx) = spawn_mock_server_202("/agent/").await;
    let server_url = format!("http://{}", addr);

    let cache = make_cache(100, 3600);
    let config_inner = Config {
        version: 1,
        agent_id: Uuid::new_v4(),
        agent_name: "Test".to_string(),
        tags: vec![],
        monitoring: MonitoringConfig::default(),
        server: ServerConfig {
            url: server_url.clone(),
            api_key: Some("test-key".to_string()),
            ..ServerConfig::default()
        },
        storage: StorageConfig {
            cache_enabled: false, // ← disabled
            ..StorageConfig::default()
        },
        endpoints: vec![],
    };
    let config = Arc::new(RwLock::new(config_inner));

    cache.push(make_ping_result("1.1.1.1")).await;
    assert_eq!(cache.len().await, 1);

    let agent_status = Arc::new(RwLock::new(smotra::AgentStatus::default()));
    let (_shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

    // Reporter should exit immediately when cache_enabled = false
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        smotra::run_result_reporter(config, cache.clone(), agent_status, shutdown_rx),
    )
    .await
    .expect("run_result_reporter should exit quickly when cache disabled");

    assert!(result.is_ok());
    // Nothing should have been sent or drained
    assert_eq!(cache.len().await, 1, "cache must be intact when disabled");
}

#[tokio::test]
async fn result_reporter_noop_on_empty_cache() {
    let (addr, _body_rx) = spawn_mock_server_202("/agent/").await;
    let server_url = format!("http://{}", addr);

    let cache = make_cache(100, 3600); // empty cache
    let config = make_config(&server_url);
    config.write().storage.cache_report_interval_secs = 1;

    let agent_status = Arc::new(RwLock::new(smotra::AgentStatus::default()));
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

    let reporter_task = tokio::spawn({
        let cache = Arc::clone(&cache);
        let config = Arc::clone(&config);
        let status = Arc::clone(&agent_status);
        async move { smotra::run_result_reporter(config, cache, status, shutdown_rx).await }
    });

    // Wait one interval, then shut down
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(2), reporter_task).await;

    // No errors should have been recorded and cache stays empty
    assert_eq!(cache.len().await, 0);
    assert_eq!(agent_status.read().failed_report_count, 0);
}
