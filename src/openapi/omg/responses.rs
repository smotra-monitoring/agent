//!
//! Response types for reusable OpenAPI response components
//!
//! This module defines response wrapper types that correspond to reusable
//! response components defined in `components/responses` in the OpenAPI spec.
//!
//! The `omg` code generator only creates types for inline response definitions,
//! so we manually define these to maintain the flexibility of reusable responses
//! in the OpenAPI specification.
//!
//! ## When to Add Types Here
//!
//! Add a new response type when:
//! 1. You add an endpoint that uses `$ref: '#/components/responses/...'`
//! 2. The response isn't already covered by an existing type
//!
//! ## Naming Convention
//!
//! Response types follow the pattern: `{OperationId}Response{StatusCode}`
//!
//! Example:
//! - Operation: `claimAgent` (operationId in OpenAPI spec)
//! - Status: `400`
//! - Type name: `ClaimAgentResponse400`
//!
//! ## Template
//!
//! /// {Description from OpenAPI spec}
//! ///
//! /// Corresponds to: `$ref: '#/components/responses/{ResponseName}'`
//! /// Used by: {OperationId} {Method} {Path}
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct {OperationId}Response{StatusCode} {
//!     pub body: {SchemaType},
//! }
//!
//! See [docs/OPENAPI_CODE_GENERATION.md] for more details.

use super::generated::models::{AgentCacheStats, AgentStatus, Error};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================
// Agent Claim Endpoint Responses
// ============================================

/// Bad request - Invalid parameters
///
/// Corresponds to: `$ref: '#/components/responses/BadRequest'`
/// Used by: claimAgent POST /agent/claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPResponse400 {
    pub body: Error,
}

/// Unauthorized - Invalid or missing authentication
///
/// Corresponds to: `$ref: '#/components/responses/Unauthorized'`
/// Used by: claimAgent POST /agent/claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPResponse401 {
    pub body: Error,
}

// ============================================
// Add more endpoint-specific response types below
// ============================================

// ============================================
// AgentStatus constructor
// ============================================

impl Default for AgentStatus {
    fn default() -> Self {
        Self {
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            config_version: 0,
            is_running: false,
            started_at: DateTime::<Utc>::UNIX_EPOCH,
            stopped_at: None,
            checks_performed: 0,
            checks_successful: 0,
            checks_failed: 0,
            reported_at: DateTime::<Utc>::UNIX_EPOCH,
            failed_report_count: 0,
            server_connected: false,
            cache_stats: AgentCacheStats {
                len: 0,
                capacity: 0,
            },
        }
    }
}

impl AgentStatus {
    /// Create a new `AgentStatus` using the compiled package version
    /// and UNIX_EPOCH as sentinel timestamps (never started/reported).
    pub fn new() -> Self {
        Default::default()
    }
}

// Example for a new endpoint:
//
// /// Description
// ///
// /// Corresponds to: `$ref: '#/components/responses/{ResponseName}'`
// /// Used by: {operationId} {METHOD} {path}
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct {OperationId}Response{StatusCode} {
//     pub body: {SchemaType},
// }
