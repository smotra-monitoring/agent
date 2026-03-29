//! # Result Cache Demo (`003_result_cache_demo`)
//!
//! Demonstrates the in-memory result cache introduced in the `cache` feature.
//!
//! What this example shows:
//! - Creating a `ResultCache` with a capacity cap and TTL
//! - Pushing `MonitoringResult` items as the monitor produces them
//! - Reading a batch with `peek_batch` (non-destructive)
//! - Confirming a successful send with `drain_front`
//! - Observing automatic TTL and size-cap eviction
//! - Reading `CacheStats`
//!
//! See `docs/features/RESULT_CACHE.md` for the full design document.

use smotra::{
    CacheStats, CheckType, Endpoint, HttpGetResult, MonitoringResult, PingResult, ResultCache,
    TcpConnectResult,
};

use std::time::Duration;
use std::sync::Arc;
use uuid::Uuid;

/// Build a simple ping result for demonstration purposes.
fn make_ping(address: &str) -> MonitoringResult {
    MonitoringResult {
        id: Uuid::new_v4(),
        agent_id: Uuid::new_v4(),
        target: Endpoint::new(address),
        check_type: CheckType::Ping(PingResult {
            successes: 3,
            failures: 0,
            success_latencies: vec![12.0, 11.5, 11.8],
            errors: vec![],
            avg_response_time_ms: Some(11.8),
            resolved_ip: Some(address.to_string()),
        }),
        timestamp: chrono::Utc::now(),
    }
}

/// Build an HTTP result for demonstration purposes.
fn make_http(url: &str) -> MonitoringResult {
    MonitoringResult {
        id: Uuid::new_v4(),
        agent_id: Uuid::new_v4(),
        target: Endpoint::new(url).with_port(443).with_tags(vec!["web".to_string()]),
        check_type: CheckType::HttpGet(HttpGetResult {
            status_code: Some(200),
            response_time_ms: Some(85.0),
            response_size_bytes: Some(1024),
            error: None,
            success: true,
        }),
        timestamp: chrono::Utc::now(),
    }
}

/// Build a TCP result to show a failure case.
fn make_tcp_fail(address: &str, port: u16) -> MonitoringResult {
    MonitoringResult {
        id: Uuid::new_v4(),
        agent_id: Uuid::new_v4(),
        target: Endpoint::new(address).with_port(port),
        check_type: CheckType::TcpConnect(TcpConnectResult {
            connected: false,
            connect_time_ms: None,
            error: Some("Connection refused".to_string()),
            resolved_ip: Some(address.to_string()),
        }),
        timestamp: chrono::Utc::now(),
    }
}

#[tokio::main]
async fn main() {
    println!("=== Result Cache Demo ===\n");

    // ─────────────────────────────────────────────────────────────
    // 1. Create a cache with a hard cap of 5 items and a 1-hour TTL.
    //    In production you'd read these from StorageConfig.
    // ─────────────────────────────────────────────────────────────
    let max_items = 5usize;
    let ttl = Duration::from_secs(3600); // 1 hour
    let cache = Arc::new(ResultCache::new(max_items, ttl));

    println!("Cache created: max_items={max_items}, ttl={ttl:?}\n");

    // ─────────────────────────────────────────────────────────────
    // 2. Push a variety of results (simulating the monitor loop).
    // ─────────────────────────────────────────────────────────────
    println!("--- Pushing 3 results ---");
    cache.push(make_ping("8.8.8.8")).await;
    cache.push(make_http("https://example.com")).await;
    cache.push(make_tcp_fail("10.0.0.99", 443)).await;

    let stats: CacheStats = cache.stats().await;
    println!("After 3 pushes  → len={}, capacity={}", stats.len, stats.capacity);

    // ─────────────────────────────────────────────────────────────
    // 3. Peek a batch of up to 10 items — non-destructive.
    // ─────────────────────────────────────────────────────────────
    println!("\n--- Peek batch (n=10, only 3 in cache) ---");
    let batch = cache.peek_batch(10).await;
    println!("Peeked {} items (cache still has {})", batch.len(), cache.len().await);
    for (i, r) in batch.iter().enumerate() {
        println!("  [{}] is_successful={}", i, r.is_successful());
    }

    // Simulate a successful POST to the server:
    println!("\n--- Simulating successful POST → draining {} items ---", batch.len());
    cache.drain_front(batch.len()).await;
    println!("Drained {} items. Cache len = {}", batch.len(), cache.len().await);

    // ─────────────────────────────────────────────────────────────
    // 4. Demonstrate size-cap eviction (FIFO).
    //    Push 6 items into a cap-5 cache — oldest should disappear.
    // ─────────────────────────────────────────────────────────────
    println!("\n--- Size-cap eviction: push 6 into a cap-5 cache ---");
    for i in 0..6usize {
        cache.push(make_ping(&format!("192.168.1.{}", i + 1))).await;
    }
    let stats = cache.stats().await;
    println!(
        "After 6 pushes: len={} (max={}, {} evicted by size)",
        stats.len,
        stats.capacity,
        6usize.saturating_sub(stats.len),
    );

    // ─────────────────────────────────────────────────────────────
    // 5. Demonstrate TTL eviction.
    //    A zero-TTL cache evicts everything on the next push.
    // ─────────────────────────────────────────────────────────────
    println!("\n--- TTL eviction: zero-TTL cache ---");
    let ttl_cache = ResultCache::new(100, Duration::from_millis(0));
    ttl_cache.push(make_ping("1.1.1.1")).await;
    println!("After push to zero-TTL cache: len={}", ttl_cache.len().await);
    // Pushing another item triggers TTL sweep of existing items.
    ttl_cache.push(make_ping("1.0.0.1")).await;
    println!("After second push: len={} (first item TTL-evicted)", ttl_cache.len().await);

    // ─────────────────────────────────────────────────────────────
    // 6. Retry semantics: peek, simulate failure, peek again.
    //    Items must not be lost on a failed POST.
    // ─────────────────────────────────────────────────────────────
    let retry_cache = ResultCache::new(100, Duration::from_secs(3600));
    retry_cache.push(make_ping("10.0.0.1")).await;
    retry_cache.push(make_ping("10.0.0.2")).await;

    println!("\n--- Peek-then-drain retry semantics ---");
    let first_peek = retry_cache.peek_batch(2).await;
    println!("First peek: {} items, cache len = {}", first_peek.len(), retry_cache.len().await);

    // Simulate server failure — do NOT drain:
    println!("Server failure: NOT draining. Cache len = {}", retry_cache.len().await);

    // On next tick, peek again — same items visible:
    let second_peek = retry_cache.peek_batch(2).await;
    println!("Second peek: {} items still available for retry", second_peek.len());

    // Now simulate success:
    retry_cache.drain_front(second_peek.len()).await;
    println!("Success: drained. Cache len = {}", retry_cache.len().await);

    // ─────────────────────────────────────────────────────────────
    // 7. Final stats
    // ─────────────────────────────────────────────────────────────
    println!("\n--- Final stats on main cache ---");
    let s = cache.stats().await;
    println!("  len      = {}", s.len);
    println!("  capacity = {}", s.capacity);

    println!("\nDone.");
}
