//! Configuration management for the agent
//! - Agent self-registration and claiming workflow

mod loader;
mod reload;
mod server_config;
mod types;

pub use reload::{handle_sighup, ConfigReloadManager, ReloadTrigger};
pub use server_config::{ClaimConfig, ServerConfig};
pub use types::*;
