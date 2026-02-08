//! Configuration management for the agent
//! - Agent self-registration and claiming workflow

mod loader;
pub mod server_config;
mod types;

pub use server_config::{ClaimConfig, ServerConfig};
pub use types::*;
