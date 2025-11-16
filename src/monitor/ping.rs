//! ICMP ping monitoring

use crate::core::{CheckType, Endpoint, MonitoringResult};
use crate::error::{Error, Result};
use chrono::Utc;
use std::net::{IpAddr, ToSocketAddrs};
use std::time::Duration;
use surge_ping::{Client, Config, PingIdentifier, PingSequence};
use tracing::debug;
use tracing_subscriber::field::debug;
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
                return MonitoringResult {
                    id: Uuid::new_v4(),
                    agent_id: agent_id.to_string(),
                    target: endpoint.clone(),
                    check_type: CheckType::Ping,
                    success: false,
                    response_time_ms: None,
                    error: Some(format!("Failed to resolve address: {}", e)),
                    timestamp: Utc::now(),
                    metadata: std::collections::HashMap::new(),
                };
            }
        };

        // Perform multiple pings
        let mut successes = 0;
        let mut total_time = 0.0;
        let mut errors = Vec::new();

        for seq in 0..self.count {
            match self.ping_once(addr, seq as u16).await {
                Ok(rtt) => {
                    // debug!("Ping reply from {}: time={:?}", addr, rtt);
                    successes += 1;
                    total_time += rtt.as_millis() as f64;
                }
                Err(e) => {
                    // debug!("Ping error from {}: {}", addr, e);
                    errors.push(e.to_string());
                }
            }
        }

        let success = successes > 0;
        let avg_time = if successes > 0 {
            Some(total_time / successes as f64)
        } else {
            None
        };

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("ping_count".to_string(), self.count.to_string());
        metadata.insert("successes".to_string(), successes.to_string());
        metadata.insert("resolved_ip".to_string(), addr.to_string());

        debug!(
            "Ping check to {} ({}): {}/{} success, avg_time={:?} ms",
            endpoint.address, addr, successes, self.count, avg_time
        );

        MonitoringResult {
            id: Uuid::new_v4(),
            agent_id: agent_id.to_string(),
            target: endpoint.clone(),
            check_type: CheckType::Ping,
            success,
            response_time_ms: avg_time,
            error: if success {
                None
            } else {
                Some(errors.join(": "))
            },
            timestamp: Utc::now(),
            metadata,
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
        .map_err(|e| Error::Network(format!("Resolution failed: {}", e)))?
        .map_err(|e| Error::Network(format!("Resolution failed: {}", e)))?;

        debug!("DNS resolution {} to {:?}", address, debug(&addrs));

        addrs
            .first()
            .map(|addr| addr.ip())
            .ok_or_else(|| Error::Network(format!("Could not resolve address: {}", address)))
    }
}
