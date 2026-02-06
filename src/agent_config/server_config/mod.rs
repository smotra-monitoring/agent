//! Server configuration module

mod persistence;
mod types;

pub use persistence::save_api_key_to_config;
pub use types::{ClaimConfig, ServerConfig};
