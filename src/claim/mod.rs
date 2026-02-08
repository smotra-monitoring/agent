//! Agent claiming workflow module

mod display;
mod polling;
mod registration;
mod token;
mod types;
mod workflow;

// Public API - expose the Claim workflow orchestrator and result type
pub use types::ClaimResult;
pub use workflow::Claim;

