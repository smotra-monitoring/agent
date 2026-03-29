//! Batch result reporter — drains the in-memory `ResultCache` and POSTs batches
//! to `POST /agent/{agentId}/results` on the central server.
//!
//! # Wire format
//!
//! The JSON payload matches `openapi::BatchMonitoringResults` (generated from
//! the OpenAPI spec). Because the OMG generator produces a `Type` discriminator
//! enum with only a `Ping` variant (a known generator limitation), we use
//! private adapter types here to correctly serialise every `CheckType` variant
//! with the `{type, result}` envelope the spec requires.
//!
//! # Peek-then-drain semantics
//!
//! 1. `peek_batch(n)` — clone the first N items without removing them.
//! 2. POST the batch.
//! 3. `drain_front(n)` on success only — guarantees at-least-once delivery.
//!    On failure the same items will be retried on the next tick.

use crate::agent_config::Config;
use crate::cache::ResultCache;
use crate::core::{AgentStatus, CheckType, Endpoint, MonitoringResult};
use crate::error::{Error, Result};
use parking_lot::RwLock;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{debug, error, info};

// ============================================================
// Wire-format adapter types
// ============================================================
//
// These private types serialise domain MonitoringResult to the wire format
// defined by the OpenAPI spec:
//   MonitoringResult.check_type → {"type": "<variant>", "result": {...}}
//
// We cannot use `openapi::CheckType` directly because the OMG generator's
// `Type` discriminator enum only contains `Ping`.

/// Adapter for `crate::core::Endpoint` — omits the domain-only fields and
/// matches the spec's `Endpoint` schema (id is absent in agent-generated events).
#[derive(Debug, Serialize)]
struct ApiEndpoint<'a> {
    address: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    enabled: bool,
    tags: &'a [String],
}

impl<'a> From<&'a Endpoint> for ApiEndpoint<'a> {
    fn from(e: &'a Endpoint) -> Self {
        Self {
            address: &e.address,
            port: e.port,
            enabled: e.enabled,
            tags: &e.tags,
        }
    }
}

/// Adjacently-tagged adapter for `crate::core::CheckType`.
///
/// `#[serde(tag = "type", content = "result")]` generates:
///   `{"type": "ping",       "result": {...}}`
///   `{"type": "traceroute", "result": {...}}`
///   etc.
///
/// This matches the spec's `CheckType` oneOf discriminator exactly.
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "result", rename_all = "lowercase")]
enum ApiCheckType<'a> {
    Ping(&'a crate::core::PingResult),
    Traceroute(&'a crate::core::TracerouteResult),
    TcpConnect(&'a crate::core::TcpConnectResult),
    UdpConnect(&'a crate::core::UdpConnectResult),
    HttpGet(&'a crate::core::HttpGetResult),
    Plugin(&'a crate::core::PluginResult),
}

impl<'a> From<&'a CheckType> for ApiCheckType<'a> {
    fn from(ct: &'a CheckType) -> Self {
        match ct {
            CheckType::Ping(r) => ApiCheckType::Ping(r),
            CheckType::Traceroute(r) => ApiCheckType::Traceroute(r),
            CheckType::TcpConnect(r) => ApiCheckType::TcpConnect(r),
            CheckType::UdpConnect(r) => ApiCheckType::UdpConnect(r),
            CheckType::HttpGet(r) => ApiCheckType::HttpGet(r),
            CheckType::Plugin(r) => ApiCheckType::Plugin(r),
        }
    }
}

/// Adapter for a single `MonitoringResult` in wire format.
#[derive(Debug, Serialize)]
struct ApiMonitoringResult<'a> {
    id: &'a uuid::Uuid,
    agent_id: &'a uuid::Uuid,
    target: ApiEndpoint<'a>,
    check_type: ApiCheckType<'a>,
    timestamp: &'a chrono::DateTime<chrono::Utc>,
}

impl<'a> From<&'a MonitoringResult> for ApiMonitoringResult<'a> {
    fn from(r: &'a MonitoringResult) -> Self {
        Self {
            id: &r.id,
            agent_id: &r.agent_id,
            target: ApiEndpoint::from(&r.target),
            check_type: ApiCheckType::from(&r.check_type),
            timestamp: &r.timestamp,
        }
    }
}

/// Wire payload matching `openapi::BatchMonitoringResults`.
#[derive(Debug, Serialize)]
struct ApiBatchPayload<'a> {
    results: Vec<ApiMonitoringResult<'a>>,
}

impl<'a> ApiBatchPayload<'a> {
    fn from_slice(batch: &'a [MonitoringResult]) -> Self {
        Self {
            results: batch.iter().map(ApiMonitoringResult::from).collect(),
        }
    }
}

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
    let payload = ApiBatchPayload::from_slice(batch);

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

    Ok(())
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

    fn make_result() -> MonitoringResult {
        MonitoringResult {
            id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            target: Endpoint::new("1.2.3.4"),
            check_type: CheckType::Ping(PingResult {
                resolved_ip: Some("1.2.3.4".to_string()),
                successes: 3,
                failures: 0,
                success_latencies: vec![1.0, 2.0, 3.0],
                avg_response_time_ms: Some(2.0),
                errors: vec![],
            }),
            timestamp: Utc::now(),
        }
    }

    mod adapter_serialisation_tests {
        use super::*;

        #[test]
        fn ping_serialises_with_type_result_envelope() {
            let result = make_result();
            let batch = ApiBatchPayload::from_slice(std::slice::from_ref(&result));
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
            use crate::core::TcpConnectResult;
            let mut result = make_result();
            result.check_type = CheckType::TcpConnect(TcpConnectResult {
                connected: true,
                connect_time_ms: Some(5.0),
                error: None,
                resolved_ip: None,
            });
            let batch = ApiBatchPayload::from_slice(std::slice::from_ref(&result));
            let json = serde_json::to_value(&batch).expect("serialisation should not fail");
            assert_eq!(
                json["results"][0]["check_type"]["type"].as_str(),
                Some("tcpconnect")
            );
        }

        #[test]
        fn httpget_serialises_correct_discriminator() {
            use crate::core::HttpGetResult;
            let mut result = make_result();
            result.check_type = CheckType::HttpGet(HttpGetResult {
                status_code: Some(200),
                response_time_ms: Some(100.0),
                response_size_bytes: Some(1024),
                error: None,
                success: true,
            });
            let batch = ApiBatchPayload::from_slice(std::slice::from_ref(&result));
            let json = serde_json::to_value(&batch).expect("serialisation should not fail");
            assert_eq!(
                json["results"][0]["check_type"]["type"].as_str(),
                Some("httpget")
            );
        }

        #[test]
        fn empty_batch_serialises_as_empty_results_array() {
            let batch = ApiBatchPayload::from_slice(&[]);
            let json = serde_json::to_value(&batch).expect("serialisation should not fail");
            assert_eq!(
                json["results"].as_array().map(|a| a.len()),
                Some(0)
            );
        }

        #[test]
        fn multiple_results_all_present_in_json() {
            let r1 = make_result();
            let r2 = make_result();
            let batch_data = vec![r1, r2];
            let batch = ApiBatchPayload::from_slice(&batch_data);
            let json = serde_json::to_value(&batch).expect("serialisation should not fail");
            assert_eq!(
                json["results"].as_array().map(|a| a.len()),
                Some(2)
            );
        }
    }
}
