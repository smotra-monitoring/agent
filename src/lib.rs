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

mod config;
mod core;
mod error;

pub mod monitor;
pub mod plugin;
pub mod reporter;

pub use config::{Config, MonitoringConfig};
pub use core::{Agent, AgentStatus, CheckType, Endpoint, MonitoringResult};
pub use error::{Error, Result};
