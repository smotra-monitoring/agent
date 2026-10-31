---
applyTo: "docs/**,examples/**,**/*.md"
---

# Documentation Requirements

**IMPORTANT**: All project documentation MUST live in the `docs/` folder. Do not create documentation files in the project root or other directories unless specifically required (e.g., `docs/README.md` serves as the main README).

## Location Rules
- **Primary location**: `docs/`
- **Feature docs**: `docs/features/<FEATURE_NAME>.md`
- **No root README**: use `docs/README.md`
- **API Docs**: OpenAPI spec stays in `api/openapi/api/spec.yaml`, referenced from `docs/`
- **Cross-references**: use relative paths from within `docs/` (e.g., `[OpenAPI Spec](../api/openapi/api/spec.yaml)`)

## Required Documentation Coverage
- Project overview and architecture
- Installation (from source and Docker)
- Configuration options and examples
- Usage — agent daemon and CLI
- Plugin development guide
- API documentation for the library
- OpenAPI specification for server API
- Contribution guidelines (`docs/CONTRIBUTING.md`)
- Testing guidelines and requirements
- Troubleshooting and FAQ

## Feature Documentation

**CRITICAL**: Every new feature MUST have a corresponding doc in `docs/features/`.

Each feature doc must include:
- **Overview** — what it does and why it exists
- **Usage examples** — complete, runnable code examples
- **Configuration** — relevant `config.toml` options
- **Integration** — how it interacts with other parts of the system
- **Edge cases and pitfalls** — known limitations, common mistakes
- **API reference** — key types and functions

### Template

````markdown
# Feature Name

## Overview
Brief description.

## Usage

### Basic Example
```rust
use smotra_agent::feature;
fn main() { /* ... */ }
```

### Configuration
```toml
[feature]
option = "value"
```

### Advanced Usage
// Edge cases, integration scenarios

## API Reference
// Key types and functions

## Common Pitfalls
// Things to watch out for
````

## Guidelines
- **Update on Change**: When modifying a feature, update its docs and examples simultaneously.
- **Test Examples**: Ensure all code examples in docs compile and run correctly.
- **Keep Current**: Examples must reflect the current API and best practices.
- **Link from Code**: Add doc comments in Rust code referencing the detailed doc in `docs/`.

## Example Files (`examples/`)

Runnable Rust examples demonstrating library features:
- **Location**: `examples/` directory at the project root.
- **Naming**: `XXX_name.rs` — three-digit order number + descriptive name.
  - e.g. `001_basic_usage.rs`, `010_plugin_example.rs`
- **Content**: Complete, runnable program with comments explaining what it demonstrates.
- **Reference**: Link from the corresponding feature doc in `docs/features/`.
