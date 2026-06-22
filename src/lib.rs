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

mod openapi;

mod agent_config;
mod cache;
mod claim;
mod core;
mod error;

mod monitor;
mod plugin;
mod reporter;

pub mod self_updater;

pub use agent_config::{ClaimConfig, Config, MonitoringConfig, ServerConfig, StorageConfig};
pub use claim::Claim;
pub use core::{
    Agent, AgentHealthStatus, AgentHeartbeat, AgentStatus, CheckType, Endpoint, ErrorDetails,
    HttpGetCheck, HttpGetCheckType, HttpGetResult, MonitoringResult, PingCheck, PingCheckType,
    PingResult, PluginCheck, PluginCheckType, PluginResult, TcpConnectCheck, TcpConnectCheckType,
    TcpConnectResult, TracerouteCheck, TracerouteCheckType, TracerouteHop, TracerouteResult,
    UdpConnectCheck, UdpConnectCheckType, UdpConnectResult,
};
pub use error::{Error, Result};

pub use plugin::{MonitoringPlugin, PluginRegistry};
