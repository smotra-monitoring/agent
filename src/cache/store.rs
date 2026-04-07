//! In-memory result cache for buffering monitoring results before server transmission.
//!
//! The `ResultCache` buffers `MonitoringResult` items in a `VecDeque` protected
//! by a `tokio::sync::Mutex`. Eviction is TTL-based: stale entries older than
//! `max_age` are removed lazily on every `push`. A hard size cap ensures memory
//! stays bounded when results accumulate faster than they are sent.
//!
//! # Peek-then-drain semantics
//!
//! The reporter calls `peek_batch` to get a cloned slice without removing items,
//! sends the batch to the server, and only calls `drain_front` after the server
//! acknowledges the batch. This guarantees at-least-once delivery: if the server
//! call fails, the same items will be retried on the next tick.
//!
//! # Memory sizing
//!
//! Each `MonitoringResult` is roughly 200–600 bytes in memory. With the default
//! cap of 10 000 items that is ≤ 6 MB. If you raise the cap significantly
//! (e.g. to cover weeks of data without a server connection) budget accordingly.

use crate::core::MonitoringResult;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, trace};

/// An entry in the cache paired with the wall-clock time it was inserted.
#[derive(Debug, Clone)]
struct CacheEntry {
    result: MonitoringResult,
    /// Stopwatch (not a timestamp) of the moment when the entry was inserted.
    /// Used only for the cache TTL eviction.
    inserted_at: Instant,
}

/// Summary statistics for cache introspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheStats {
    /// Number of entries currently held in the cache.
    pub len: usize,
    /// Maximum number of entries the cache will hold.
    pub capacity: usize,
}

/// Thread-safe in-memory cache for `MonitoringResult` items.
#[derive(Debug, Clone)]
pub struct ResultCache {
    inner: Arc<Mutex<VecDeque<CacheEntry>>>,
    max_size: usize,
    max_age: Duration,
}

impl ResultCache {
    /// Create a new `ResultCache`.
    ///
    /// # Arguments
    ///
    /// * `max_size` – Maximum number of results to hold. When the hard cap is
    ///   reached the oldest entry is dropped to make room for the new one.
    /// * `max_age` – How long a result is considered fresh. Stale entries are
    ///   removed lazily before each new insertion.
    pub fn new(max_size: usize, max_age: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(
                max_size.min(4096), // pre-allocate a reasonable chunk
            ))),
            max_size,
            max_age,
        }
    }

    /// Push a new result into the cache.
    ///
    /// Before inserting:
    /// 1. TTL eviction removes all entries older than `max_age`.
    /// 2. If still at `max_size`, the oldest entry is dropped (FIFO).
    pub async fn push(&self, result: MonitoringResult) {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();

        // Lazy TTL eviction: drop entries from the front that are too old.
        while let Some(front) = inner.front() {
            if now.duration_since(front.inserted_at) > self.max_age {
                inner.pop_front();
            } else {
                break;
            }
        }

        // Hard size cap: drop oldest if at capacity.
        if self.max_size > 0 && inner.len() >= self.max_size {
            inner.pop_front();
            debug!(
                "Cache at capacity ({}), evicted oldest entry",
                self.max_size
            );
        }

        trace!(
            result_id = %result.id,
            target = %result.target.address,
            "Caching monitoring result"
        );

        inner.push_back(CacheEntry {
            result,
            inserted_at: now,
        });
    }

    /// Return a cloned slice of the first `n` entries without removing them.
    ///
    /// Used by the reporter to build a batch payload before attempting a send.
    /// Items are only removed via `drain_front` after the server acknowledges.
    pub async fn peek_batch(&self, n: usize) -> Vec<MonitoringResult> {
        let inner = self.inner.lock().await;
        inner.iter().take(n).map(|e| e.result.clone()).collect()
    }

    /// Remove the first `n` entries from the front of the queue.
    ///
    /// Call this after a successful server transmission to confirm delivery.
    /// If `n` exceeds the current length, all entries are drained.
    pub async fn drain_front(&self, n: usize) {
        let mut inner = self.inner.lock().await;
        let to_drain = n.min(inner.len());
        for _ in 0..to_drain {
            inner.pop_front();
        }
        debug!(
            "Drained {} entries from cache, {} remaining",
            to_drain,
            inner.len()
        );
    }

    /// Return summary statistics for cache introspection.
    pub async fn stats(&self) -> CacheStats {
        CacheStats {
            len: self.inner.lock().await.len(),
            capacity: self.max_size,
        }
    }

    /// Return the current number of entries in the cache.
    #[cfg(test)]
    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{CheckType, Endpoint, MonitoringResult, PingResult};
    use chrono::Utc;
    use uuid::Uuid;

    fn make_result(address: &str) -> MonitoringResult {
        use crate::core::{PingCheck, PingCheckType};
        MonitoringResult {
            id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            target: Endpoint::new(address),
            check_type: CheckType::PingCheck(PingCheck {
                r#type: PingCheckType::Ping,
                result: PingResult {
                    resolved_ip: None,
                    successes: Some(1),
                    failures: Some(0),
                    success_latencies: Some(vec![1.0]),
                    avg_response_time_ms: Some(1.0),
                    errors: Some(vec![]),
                },
            }),
            timestamp: Utc::now(),
        }
    }

    mod push_tests {
        use super::*;

        #[tokio::test]
        async fn push_single_result() {
            let cache = ResultCache::new(100, Duration::from_secs(3600));
            cache.push(make_result("1.2.3.4")).await;
            assert_eq!(cache.len().await, 1);
        }

        #[tokio::test]
        async fn push_multiple_results() {
            let cache = ResultCache::new(100, Duration::from_secs(3600));
            for i in 0..5 {
                cache.push(make_result(&format!("10.0.0.{}", i))).await;
            }
            assert_eq!(cache.len().await, 5);
        }

        #[tokio::test]
        async fn push_evicts_oldest_when_at_capacity() {
            let cache = ResultCache::new(3, Duration::from_secs(3600));
            let r1 = make_result("1.1.1.1");
            let r2 = make_result("2.2.2.2");
            let r3 = make_result("3.3.3.3");
            let r4 = make_result("4.4.4.4");
            let first_id = r1.id;

            cache.push(r1).await;
            cache.push(r2).await;
            cache.push(r3).await;
            cache.push(r4).await;

            // Still at max_size
            assert_eq!(cache.len().await, 3);

            // The first result should have been evicted
            let batch = cache.peek_batch(3).await;
            assert!(
                !batch.iter().any(|r| r.id == first_id),
                "oldest entry should be evicted"
            );
        }

        #[tokio::test]
        async fn push_zero_max_size_means_no_size_cap() {
            // max_size = 0 is treated as "no size cap" (unlimited).
            // Use cache_enabled = false (via StorageConfig) to disable caching entirely.
            let cache = ResultCache::new(0, Duration::from_secs(3600));
            for i in 0..100 {
                cache
                    .push(make_result(&format!("10.0.{}.{}", i / 256, i % 256)))
                    .await;
            }
            assert_eq!(
                cache.len().await,
                100,
                "max_size=0 means unlimited, not disabled"
            );
        }
    }

    mod ttl_eviction_tests {
        use super::*;

        #[tokio::test]
        async fn ttl_eviction_removes_stale_entries_on_push() {
            // Use a very short TTL
            let cache = ResultCache::new(100, Duration::from_millis(1));
            cache.push(make_result("1.1.1.1")).await;

            // Wait for the entry to become stale
            tokio::time::sleep(Duration::from_millis(10)).await;

            // Push a new entry — this should trigger eviction of stale entries
            cache.push(make_result("2.2.2.2")).await;

            // Only the fresh entry should remain
            assert_eq!(cache.len().await, 1);
            let batch = cache.peek_batch(10).await;
            assert_eq!(batch[0].target.address, "2.2.2.2");
        }

        #[tokio::test]
        async fn fresh_entries_not_evicted_before_ttl() {
            let cache = ResultCache::new(100, Duration::from_secs(3600));
            cache.push(make_result("1.1.1.1")).await;
            cache.push(make_result("2.2.2.2")).await;
            assert_eq!(cache.len().await, 2);
        }
    }

    mod peek_tests {
        use super::*;

        #[tokio::test]
        async fn peek_batch_returns_first_n() {
            let cache = ResultCache::new(100, Duration::from_secs(3600));
            for i in 0..10 {
                cache.push(make_result(&format!("10.0.0.{}", i))).await;
            }
            let batch = cache.peek_batch(3).await;
            assert_eq!(batch.len(), 3);
            assert_eq!(batch[0].target.address, "10.0.0.0");
            assert_eq!(batch[1].target.address, "10.0.0.1");
            assert_eq!(batch[2].target.address, "10.0.0.2");
        }

        #[tokio::test]
        async fn peek_batch_does_not_remove_entries() {
            let cache = ResultCache::new(100, Duration::from_secs(3600));
            cache.push(make_result("1.1.1.1")).await;
            cache.peek_batch(1).await;
            cache.peek_batch(1).await;
            assert_eq!(cache.len().await, 1, "peek must not remove entries");
        }

        #[tokio::test]
        async fn peek_batch_larger_than_cache_returns_all() {
            let cache = ResultCache::new(100, Duration::from_secs(3600));
            cache.push(make_result("1.1.1.1")).await;
            cache.push(make_result("2.2.2.2")).await;
            let batch = cache.peek_batch(999).await;
            assert_eq!(batch.len(), 2);
        }

        #[tokio::test]
        async fn peek_empty_cache_returns_empty_vec() {
            let cache = ResultCache::new(100, Duration::from_secs(3600));
            let batch = cache.peek_batch(10).await;
            assert!(batch.is_empty());
        }
    }

    mod drain_tests {
        use super::*;

        #[tokio::test]
        async fn drain_front_removes_n_entries() {
            let cache = ResultCache::new(100, Duration::from_secs(3600));
            for i in 0..5 {
                cache.push(make_result(&format!("10.0.0.{}", i))).await;
            }
            cache.drain_front(3).await;
            assert_eq!(cache.len().await, 2);
            let remaining = cache.peek_batch(2).await;
            assert_eq!(remaining[0].target.address, "10.0.0.3");
            assert_eq!(remaining[1].target.address, "10.0.0.4");
        }

        #[tokio::test]
        async fn drain_front_more_than_present_drains_all() {
            let cache = ResultCache::new(100, Duration::from_secs(3600));
            cache.push(make_result("1.1.1.1")).await;
            cache.drain_front(999).await;
            assert_eq!(cache.len().await, 0);
        }

        #[tokio::test]
        async fn drain_empty_cache_is_noop() {
            let cache = ResultCache::new(100, Duration::from_secs(3600));
            cache.drain_front(5).await; // should not panic
            assert_eq!(cache.len().await, 0);
        }

        #[tokio::test]
        async fn peek_then_drain_semantics() {
            let cache = ResultCache::new(100, Duration::from_secs(3600));
            let r1 = make_result("1.1.1.1");
            let r2 = make_result("2.2.2.2");
            let id1 = r1.id;
            cache.push(r1).await;
            cache.push(r2).await;

            let batch = cache.peek_batch(1).await;
            assert_eq!(batch[0].id, id1);
            assert_eq!(cache.len().await, 2, "peek must not remove");

            cache.drain_front(1).await;
            assert_eq!(cache.len().await, 1, "drain should remove one");
            let remaining = cache.peek_batch(1).await;
            assert_ne!(remaining[0].id, id1, "first entry must be gone after drain");
        }
    }

    mod stats_tests {
        use super::*;

        #[tokio::test]
        async fn stats_reflect_current_state() {
            let cache = ResultCache::new(50, Duration::from_secs(3600));
            assert_eq!(
                cache.stats().await,
                CacheStats {
                    len: 0,
                    capacity: 50
                }
            );
            cache.push(make_result("1.1.1.1")).await;
            cache.push(make_result("2.2.2.2")).await;
            assert_eq!(
                cache.stats().await,
                CacheStats {
                    len: 2,
                    capacity: 50
                }
            );
        }
    }

    mod concurrency_tests {
        use super::*;
        use std::sync::Arc as StdArc;

        #[tokio::test]
        async fn concurrent_pushes_stay_within_capacity() {
            let max_size = 1000;
            let cache = StdArc::new(ResultCache::new(max_size, Duration::from_secs(3600)));
            let mut handles = vec![];
            for i in 0..max_size * 2 {
                let c = StdArc::clone(&cache);
                handles.push(tokio::spawn(async move {
                    c.push(make_result(&format!("10.0.{}.{}", i / 256, i % 256)))
                        .await;
                }));
            }
            for h in handles {
                h.await.unwrap();
            }
            // Must not exceed max_size
            assert!(
                cache.len().await == max_size,
                "cache must not exceed max_size"
            );
        }

        #[tokio::test]
        async fn concurrent_peek_and_drain_are_consistent() {
            let cache = StdArc::new(ResultCache::new(50, Duration::from_secs(3600)));
            for i in 0..50u32 {
                cache.push(make_result(&format!("10.0.0.{}", i))).await;
            }

            let cache_clone = StdArc::clone(&cache);
            let drain_task = tokio::spawn(async move {
                cache_clone.drain_front(20).await;
            });

            let batch = cache.peek_batch(50).await;
            drain_task.await.unwrap();

            // After draining 20, remaining should be ≤ 30
            assert!(cache.len().await == 30);
            // batch captured before or during drain; length is 0–50
            assert!(batch.len() == 50);
        }
    }
}
