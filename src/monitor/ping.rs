//! ICMP ping monitoring

use crate::core::{CheckType, Endpoint, MonitoringResult, PingResult};
use crate::error::{Error, Result};
use chrono::Utc;
use std::net::{IpAddr, ToSocketAddrs};
use std::time::Duration;
use surge_ping::{Client, Config, PingIdentifier, PingSequence};
use tracing::debug;
use uuid::Uuid;

/// Ping checker for ICMP reachability tests
pub struct PingChecker {
    client: Client,
    timeout: Duration,
    count: u32,
}

impl PingChecker {
    /// Create a new ping checker
    pub fn new(timeout: Duration, count: u32) -> Result<Self> {
        let config = Config::default();
        let client = Client::new(&config)
            .map_err(|e| Error::Network(format!("Failed to create ping client: {}", e)))?;

        Ok(Self {
            client,
            timeout,
            count,
        })
    }

    /// Perform a ping check on the given endpoint
    pub async fn check(&self, agent_id: &str, endpoint: &Endpoint) -> MonitoringResult {
        // Resolve the address
        let addr = match self.resolve_address(&endpoint.address).await {
            Ok(addr) => addr,
            Err(e) => {
                let ping_result = PingResult {
                    successes: 0,
                    failures: 1,
                    success_latencies: Vec::new(),
                    errors: vec![format!("Failed to resolve address: {}", e)],
                    avg_response_time_ms: None,
                    resolved_ip: None,
                };

                return MonitoringResult {
                    id: Uuid::new_v4(),
                    agent_id: agent_id.to_string(),
                    target: endpoint.clone(),
                    check_type: CheckType::Ping(ping_result),
                    timestamp: Utc::now(),
                    metadata: std::collections::HashMap::new(),
                };
            }
        };

        // Perform multiple pings
        let mut successes = 0;
        let mut failures = 0;
        let mut success_latencies = Vec::new();
        let mut errors = Vec::new();

        for seq in 0..self.count {
            match self.ping_once(addr, seq as u16).await {
                Ok(rtt) => {
                    successes += 1;
                    let latency_ms = rtt.as_millis() as f64;
                    success_latencies.push(latency_ms);
                }
                Err(e) => {
                    failures += 1;
                    errors.push(e.to_string());
                }
            }
        }

        let avg_response_time_ms = if !success_latencies.is_empty() {
            Some(success_latencies.iter().sum::<f64>() / success_latencies.len() as f64)
        } else {
            None
        };

        debug!(
            "Ping check to {} ({}): {}/{} success, avg_time={:.2?} ms",
            endpoint.address, addr, successes, self.count, avg_response_time_ms
        );

        let ping_result = PingResult {
            resolved_ip: Some(addr.to_string()),
            successes,
            failures,
            success_latencies: success_latencies.clone(),
            avg_response_time_ms,
            errors: errors.clone(),
        };

        MonitoringResult {
            id: Uuid::new_v4(),
            agent_id: agent_id.to_string(),
            target: endpoint.clone(),
            check_type: CheckType::Ping(ping_result),
            timestamp: Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Perform a single ping
    async fn ping_once(&self, addr: IpAddr, seq: u16) -> Result<Duration> {
        let payload = "******    ping   ------ 1234567890 ===== abcdefghi ____ ".as_bytes();
        let identifier = PingIdentifier(rand::random());
        let sequence = PingSequence(seq);

        let mut pinger = self.client.pinger(addr, identifier).await;

        // let start = Instant::now();

        match tokio::time::timeout(self.timeout, pinger.ping(sequence, payload)).await {
            Ok(Ok((_, duration))) => Ok(duration),
            Ok(Err(e)) => Err(Error::Network(format!("Ping failed: {}", e))),
            Err(_) => Err(Error::Network("Ping timeout".to_string())),
        }
    }

    /// Resolve a hostname or IP address to an IP address
    async fn resolve_address(&self, address: &str) -> Result<IpAddr> {
        // Try parsing as IP first
        if let Ok(ip) = address.parse::<IpAddr>() {
            return Ok(ip);
        }

        // Resolve as hostname
        let addr_str = format!("{}:0", address);
        let addrs: Vec<_> = tokio::task::spawn_blocking(move || {
            addr_str
                .to_socket_addrs()
                .map(|addrs| addrs.collect::<Vec<_>>())
        })
        .await
        .map_err(Error::JoinError)?
        .map_err(|e| Error::Network(format!("Resolution failed: {}", e)))?;

        debug!("DNS resolution {} to {:?}", address, debug(&addrs));

        addrs
            .first()
            .map(|addr| addr.ip())
            .ok_or_else(|| Error::Network(format!("Could not resolve address: {}", address)))
    }
}
