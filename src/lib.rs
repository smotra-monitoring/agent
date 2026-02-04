//! Smotra Agent Library
//!
//! A distributed monitoring agent library for tracking reachability and performance
//! of networked hosts. The library provides:
//!
//! - ICMP ping monitoring
//! - Traceroute capabilities
//! - Configurable monitoring intervals
//! - Data reporting to a central server
//! - Local caching for offline operation
//! - Plugin system for extensibility

mod agent_config;
mod core;
mod error;

pub mod monitor;
pub mod plugin;
pub mod reporter;

pub use agent_config::{Config, MonitoringConfig, ServerConfig, StorageConfig};
pub use core::{
    Agent, AgentHealthStatus, AgentHeartbeat, AgentStatus, CheckType, Endpoint, HttpGetResult,
    MonitoringResult, PingResult, PluginResult, TcpConnectResult, TracerouteHop, TracerouteResult,
    UdpConnectResult,
};
pub use error::{Error, Result};
pub use reporter::HeartbeatReporter;
