---
applyTo: "src/openapi/**,api/openapi/**"
---

# OpenAPI Specification and Type Generation

## Specification Location

The OpenAPI specification is the **single source of truth** for the API contract between agent and server:
- `api/openapi/api/spec.yaml`

## Generated Types (OMG Tool)

**CRITICAL**: All types that correspond to OpenAPI schema definitions MUST be generated from the spec using the OMG (OpenAPI Model Generator) tool. Do NOT manually create types that are defined in the OpenAPI spec.

### Generated Code Layout
- **Specification**: `./api/openapi/api/spec.yaml`
- **Generated Types**: `./src/openapi/omg/generated/`
- **Manual Extensions**: `./src/openapi/omg/responses.rs` (reusable response components not handled by OMG)
- **Module Entry**: types re-exported through `./src/openapi/mod.rs`

### Usage Pattern

Always import via the `openapi` module alias so it is immediately clear a type comes from the API spec:

```rust
use crate::openapi;

let registration = openapi::AgentRegistration {
    agent_id: agent_id.to_string(),
    claim_token_hash: hashed_token,
};

let response: openapi::ClaimAgentResponse201 = /* ... */;
```

Rules:
- **Always use the `openapi::` prefix** — distinguishes API types from domain types.
- **Never create manual parallel types** — if a type is in the spec, use the generated version.
- **Import pattern**: `use crate::openapi;` then `openapi::TypeName`.
- **Source of truth**: the OpenAPI spec defines the contract; generated types follow it.

### When to Regenerate

Regenerate after any of these:
1. Modifying `api/openapi/api/spec.yaml`
2. Adding new endpoints or response types
3. Changing existing schema definitions
4. Updating request/response structures

```bash
# Using justfile
just generate-omg

# Or manually
omg --input ./api/openapi/api/spec.yaml -o ./src/openapi/omg/generated/
echo "pub mod responses;" >> ./src/openapi/omg/mod.rs
```

### Manual Response Wrappers

OMG does not generate wrappers for `$ref: '#/components/responses/...'` entries. These must be manually maintained in `responses.rs`. See [OPENAPI_CODE_GENERATION.md](../../docs/features/OPENAPI_CODE_GENERATION.md) for details.

### Workflow Example

1. Define the type in `api/openapi/api/spec.yaml`.
2. Run `just generate-omg`.
3. Use in code:
```rust
use crate::openapi;

let heartbeat = openapi::AgentHeartbeat {
    agent_id: self.agent_id.clone(),
    timestamp: Utc::now().timestamp(),
};
```

```rust
// ❌ WRONG — never create a parallel manual struct
struct AgentHeartbeat { agent_id: String, timestamp: i64 }

// ✅ CORRECT
use crate::openapi;
let heartbeat = openapi::AgentHeartbeat { /* ... */ };
```
