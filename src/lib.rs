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

mod agent;
pub mod config;
pub mod error;
pub mod monitor;
pub mod plugin;
pub mod reporter;
pub mod types;

pub use agent::Agent;
pub use config::Config;
pub use error::{Error, Result};
pub use types::{AgentStatus, MonitoringResult};
