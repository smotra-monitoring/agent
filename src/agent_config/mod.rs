//! Configuration management for the agent
//! - Agent self-registration and claiming workflow

mod hot_reload;
mod loader;
mod reload;
mod server_config;
mod types;

pub use hot_reload::run_hot_reload;
pub use reload::{handle_sighup, ConfigReloadManager, ReloadTrigger};
pub use server_config::{ClaimConfig, ServerConfig};
pub use types::*;
