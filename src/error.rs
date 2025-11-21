//! Error types for the agent library

use thiserror::Error;

/// Result type alias for agent operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for the agent
#[derive(Debug, Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Monitoring error: {0}")]
    Monitoring(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
