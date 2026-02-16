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

use super::generated::models::Error;
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
