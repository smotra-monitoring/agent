//! Core agent types and implementation

mod agent;
pub mod types;

pub use agent::Agent;
pub use types::{
    AgentStatus, CheckType, Endpoint, MonitoringResult, TracerouteHop, TracerouteResult,
};
