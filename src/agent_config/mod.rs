//! Configuration management for the agent
//! - Agent self-registration and claiming workflow

mod hot_reload;
mod loader;
mod merge_patch;
mod openapi_ext;
mod server_sync;

pub use crate::openapi::{
    AgentConfig, ClaimConfig, MonitoringConfig, SelfUpgradeConfig, ServerConfig, StorageConfig,
};
pub use hot_reload::run_hot_reload;
pub(crate) use server_sync::sync_config_from_server;
