# Result Cache Feature

## Overview

The result cache is an in-memory buffer that decouples **monitoring frequency** from **reporting frequency**. Instead of sending each monitoring result to the server immediately, the agent accumulates them in a `ResultCache` and drains them in configurable batches at a separate, potentially slower interval.

Benefits:
- **Absorbs transient connectivity issues** — results are retained while the server is unreachable and sent once connectivity returns.
- **Reduces server load** — one POST per interval replaces potentially many individual requests.
- **Decouples timers** — you can monitor every 15 s but report every 60 s without losing data.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  Agent                                                          │
│                                                                 │
│  ┌──────────────────┐  MonitoringResult   ┌─────────────────┐  │
│  │  monitor::       │ ─────────────────►  │  ResultCache    │  │
│  │  run_monitoring  │                     │  (VecDeque)     │  │
│  └──────────────────┘                     └────────┬────────┘  │
│                                                    │           │
│                                         peek_batch │           │
│                                                    ▼           │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  reporter::run_result_reporter                           │  │
│  │  ① peek_batch(n)  → clone first N items                 │  │
│  │  ② POST /agent/{id}/results  (BatchMonitoringResults)   │  │
│  │  ③ drain_front(n) on 2xx ACK only                       │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

**Peek-then-drain semantics** guarantee at-least-once delivery: a result is only removed from the cache after the server acknowledges the POST. If the POST fails, the same items are retried on the next tick.

## Configuration

All cache settings live in the `[storage]` section of `config.toml`:

```toml
[storage]
cache_dir              = "./cache"    # reserved for future disk persistence

# ── Result cache ────────────────────────────────────────────────
max_cached_results     = 10000   # hard cap on number of items (0 = unlimited)
max_cache_age_secs     = 86400   # TTL: items older than this are evicted lazily
cache_enabled          = true    # set to false to disable caching entirely
cache_batch_size       = 100     # results per POST request
cache_report_interval_secs = 60  # seconds between drain attempts
```

### Field reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_cached_results` | `usize` | `10000` | Maximum items in memory. When full the oldest entry is dropped (FIFO). `0` disables the size cap. |
| `max_cache_age_secs` | `u64` | `86400` | TTL in seconds. Stale entries are evicted lazily on the next `push`. |
| `cache_enabled` | `bool` | `true` | Master switch. When `false` the result reporter exits immediately and nothing is cached. |
| `cache_batch_size` | `usize` | `100` | Items per `POST /agent/{id}/results` request. |
| `cache_report_interval_secs` | `u64` | `60` | Drain interval. Hot-reload aware. |

## Memory sizing

| Scenario | Endpoints | Interval | TTL | ~Items | ~RAM |
|----------|-----------|----------|-----|--------|------|
| Small | 10 | 60 s | 24 h | 1 440 | < 1 MB |
| Medium | 50 | 60 s | 24 h | 7 200 | ~4 MB |
| Default cap | — | — | — | 10 000 | ~5 MB |
| Weeks (no server) | 50 | 60 s | 4 weeks | ~2 M | ~1 GB |

> **If you intend to cache weeks of data without a server connection you will need both a large `max_cached_results` and significant RAM, or wait for the disk-persistence feature (future work).**

## Eviction policy

Two eviction mechanisms operate at each `push`:

1. **TTL eviction (front-sweep)** — entries at the front of the queue older than `max_cache_age_secs` are removed first.
2. **Size cap eviction** — if the queue is still at `max_cached_results` after TTL eviction, the single oldest entry is dropped.

Both operate lazily (on `push`), so there are no background GC tasks and no additional lock contention during quiet periods.

## Server API endpoint

Results are submitted to:

```
POST /agent/{agentId}/results
X-Agent-API-Key: <api_key>
Content-Type: application/json

{
  "results": [
    {
      "id": "...",
      "agent_id": "...",
      "target": { "address": "8.8.8.8", "enabled": true, "tags": [] },
      "check_type": { "type": "ping", "result": { "successes": 3, ... } },
      "timestamp": "2026-03-01T10:00:00Z"
    }
  ]
}
```

The server **must deduplicate** by `result.id` — since the reporter uses peek-then-drain, a network failure between POST and drain means the same batch may be retransmitted.

The endpoint is defined in [`api/openapi/api/spec.yaml`](../../api/openapi/api/spec.yaml) as `POST /agent/{agentId}/results` with operationId `submitAgentResults`.

### CheckType wire format

The `check_type` field uses an adjacently-tagged envelope `{"type": "<variant>", "result": {...}}` to match the OpenAPI spec's `CheckType` oneOf discriminator:

| Variant | `"type"` value | result schema |  
|---------|--------------|---------------|
| Ping | `"ping"` | `PingResult` |
| Traceroute | `"traceroute"` | `TracerouteResult` |
| TCP Connect | `"tcpconnect"` | `TcpConnectResult` |
| UDP Connect | `"udpconnect"` | `UdpConnectResult` |
| HTTP GET | `"httpget"` | `HttpGetResult` |
| Plugin | `"plugin"` | `PluginResult` |

> **Note**: The OMG-generated `CheckType` type has a `Type` enum with only a `Ping` variant (known generator limitation). The reporter uses private adapter types in `results.rs` to produce the correct wire format.

## AgentStatus integration

`AgentStatus.cached_reports` reflects the current cache depth and is included in every `POST /agent/report` heartbeat. Monitor this field to detect connectivity issues:

- `cached_reports > 0` and `server_connected = false` → results are accumulating; server is unreachable.
- `cached_reports = 0` and `server_connected = true` → fully caught up.

## Usage example

See [`examples/003_result_cache_demo.rs`](../../examples/003_result_cache_demo.rs) for a complete runnable demonstration.

```rust
use smotra::{ResultCache, CacheStats};
use std::time::Duration;

let cache = ResultCache::new(
    1000,                        // max_cached_results
    Duration::from_secs(86400),  // max_cache_age_secs = 24 h
);

// Push results as they arrive from the monitor:
cache.push(monitoring_result).await;

// Reporter reads a batch:
let batch = cache.peek_batch(100).await;
// ... POST to server ...
// Only drain on success:
cache.drain_front(batch.len()).await;

// Inspect current state:
let stats: CacheStats = cache.stats().await;
println!("{} items in cache (cap {})", stats.len, stats.capacity);
```

## Disabling the cache

Set `cache_enabled = false` in `[storage]` to turn off in-memory caching entirely. The `run_result_reporter` task will exit immediately and no results will be buffered. This is useful when:
- The agent has a persistent, low-latency connection to the server.
- You want to send results only via a custom plugin.

## Disk persistence (future work)

The `cache_dir` config field is reserved but not yet used. A future enhancement will serialize the `VecDeque` to disk (using `serde + bincode`) and reload it on startup, enabling recovery of buffered results after an agent restart. Track progress in [ROADMAP.md](../ROADMAP.md).

## Common pitfalls

| Symptom | Likely cause | Fix |
|---------|-------------|-----|
| Cache grows without bound | `max_cache_age_secs` is very large AND `cache_enabled = true` but server unreachable | Lower `max_cached_results` or fix server connectivity |
| Old results never sent | `cache_report_interval_secs` is very large | Reduce the interval |
| High failed_report_count | Server returning non-2xx | Check server logs; reporter retries automatically |
| Results lost on restart | Only in-memory; no disk persistence | Implement disk persistence (future work) |
