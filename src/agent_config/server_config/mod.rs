//! Server configuration module

pub mod persistence;
pub mod types;

pub use persistence::{save_api_key_to_config, update_agent_id};
pub use types::{ClaimConfig, ServerConfig};
