//! Configuration management for the agent
//! - Agent self-registration and claiming workflow

mod hot_reload;
mod loader;
mod openapi_ext;

pub use crate::openapi::{
    AgentConfig, ClaimConfig, MonitoringConfig, SelfUpgradeConfig, ServerConfig, StorageConfig,
};
pub use hot_reload::run_hot_reload;
