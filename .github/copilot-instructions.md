# Project description

This project is a distributed monitoring system designed to track reachability and performance of agents installed on various hosts. It consists of a central server that collects data from multiple agents deployed across different machines. The system provides real-time monitoring, alerting, and reporting capabilities to ensure the health and performance of the monitored infrastructure.

# Key Features
- **Agent-Based Monitoring**: Lightweight agents installed on hosts to collect metrics and send them to the central server.
- **Centralized Data Collection**: A server that aggregates data from all agents for analysis and reporting.
- **Real-Time Alerts**: Configurable alerts based on predefined thresholds to notify administrators of potential issues.
- **Performance Metrics**: Collection of various performance metrics such as reachability, response time and potentially other system metrics that can be extended via plugins.
- **Scalability**: Designed to handle a large number of agents and hosts efficiently.
- **Extensible Architecture**: Support for plugins to extend monitoring capabilities and integrate with other systems.
- **User-Friendly Interface**: A web-based dashboard for visualizing data, configuring agents, and managing alerts.
- **APIs for Integration**: RESTful APIs to allow integration with other systems and automation tools.

# Technologies Used
- Agent Development is in Rust for performance and safety.
- Server-side components are developed in Go.
- Data storage using a time-series database (PostgreSQL + TimescaleDB) for efficient metric storage and retrieval.
- Web interface built with standard web technologies (HTML, CSS, TypeScript) for a responsive user experience CSS framework (e.g. Bulima).
- Communication between agents and server using RESTful APIs over HTTP/HTTPS.
- Containerization using Docker for easy deployment and scalability.
- Orchestration using Kubernetes for managing deployments in a clustered environment.
- Monitoring and logging using Prometheus and Grafana for system health and performance visualization.

# Agent Capabilities
- Agents check reachability of other agents or predefined endpoints.
- Measure response times and log results.
- Send collected data to the central server at regular intervals.
- Support for custom plugins to extend monitoring functionality.
- Configuration management to adjust monitoring parameters remotely from the server. Must be able use local configuration if server is unreachable.
- Secure communication with the server using TLS/SSL.

Agent should be able to operate in a standalone mode if the server is unreachable, caching data locally and sending it once the connection is restored. Agents should also support auto-updates to ensure they are running the latest version. Agent use ICMP ping and traceroute for reachability checks, with options for TCP/UDP checks as plugins. 

Agent implementation should prioritize low resource usage to minimize impact on host performance. Therefore tokio async runtime is preferred for Rust implementation. 
Library for ICMP ping such as "surge-ping" can be used. 
Library for traceroute such as "traceroute-rs" can be used.

Agent configuration should include:
- List of endpoints to monitor.
- Monitoring intervals.

List of endpoints can be provided by the server or configured locally. Agents might be grouped by tags and organize itself into meshes where each agent monitors a subset of other agents based on tags.

Agent implementation is a library that can be embedded into other Rust applications. The library exposes a simple API for starting and stopping monitoring tasks, configuring endpoints, and retrieving collected data. 

Set of binaries
- `agent`: Main binary to run the monitoring agent.
- `agent-cli`: Command-line interface for managing and configuring the agent with Ratatui for TUI, will be used for debugging and local configuration.
- `agent-updater`: Binary to handle auto-updates for the agent (stub implementation).

Files structure
- `src/`: Source code for the agent library and binaries.
- `Cargo.toml`: Cargo configuration file for the project.
- `Dockerfile`: Dockerfile for building the agent container image.
- `docs/`: Documentation for the project.
- `docs/features/`: Documentation for the project's features.
- `examples/`: Example plugin implementations.
- `tests/`: Unit and integration tests for the agent library and binaries.
- `api/`: OpenAPI specification and related documentation.
- `src/lib.rs`: Main library file for the agent.
- `src/bin/`: Directory containing the main binaries for the agent.
- `src/bin/agent.rs`: Main binary to run the monitoring agent.
- `src/bin/agent_cli/`: Directory for the agent CLI with TUI implementation.
- `src/bin/agent_cli/main.rs`: Main entry point for the agent CLI.
- `src/bin/agent_cli/cli_args.rs`: Command-line argument definitions.
- `src/bin/agent_cli/commands.rs`: Command handlers for status, validation, and config generation.
- `src/bin/agent_cli/logging.rs`: Log buffer and initialization logic.
- `src/bin/agent_cli/tui/`: TUI implementation with Ratatui (mod.rs, render.rs, runner.rs, ui_loop.rs).
- `src/bin/agent_updater.rs`: Binary to handle auto-updates for the agent (stub implementation).

# Code organization

- **Module Structure**: `mod.rs` files should contain only module declarations (`mod`) and re-exports (`pub use`). No functional code (implementations, functions, structs) should be placed in `mod.rs` files - these belong in dedicated files within the module directory.
- **Visibility**: Make methods and functions private by default. Only mark items as `pub` when they are explicitly needed as part of the public API. Avoid proliferating `pub fn` unnecessarily - every public item increases the API surface and maintenance burden. Ask yourself: "Does this need to be public, or is it an implementation detail?"
- "utilities" module can be used inside other modules to declare only private support functions for the module.
- reserve "support" module for external functions that can be used by other projects in cargo workspaces

## Rust Design Patterns

Follow idiomatic Rust design patterns where applicable to improve code quality, maintainability, and API ergonomics:

### Builder Pattern
Use the Builder pattern for complex types with many optional fields or configuration options:
- Implement for structs with 4+ fields where some are optional
- Provide a clean, fluent API for constructing complex objects
- Example use cases: Configuration objects, request builders, test fixtures
```rust
// Example structure:
struct Config {
    // fields
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

struct ConfigBuilder {
    // same fields as Config, but all Option<T>
}

impl ConfigBuilder {
    pub fn field_name(mut self, value: Type) -> Self {
        self.field_name = Some(value);
        self
    }
    
    pub fn build(self) -> Result<Config, BuilderError> {
        // validation and construction
    }
}
```

### Type State Pattern
Use the Type State pattern to enforce correct API usage at compile time:
- For objects with distinct lifecycle states (e.g., Unclaimed, Claimed, Running)
- Prevents invalid state transitions through the type system
- Makes illegal states unrepresentable
```rust
// Example structure:
struct Agent<State> {
    // common fields
    state: PhantomData<State>,
}

struct Unclaimed;
struct Claimed;
struct Running;

impl Agent<Unclaimed> {
    pub fn claim(self) -> Result<Agent<Claimed>, Error> {
        // transition logic
    }
}

impl Agent<Claimed> {
    pub fn start(self) -> Result<Agent<Running>, Error> {
        // transition logic
    }
}
```

### Factory Pattern
Use Factory functions or traits for creating related types with complex initialization:
- When object creation requires coordination of multiple components
- For creating different implementations of a trait based on configuration
- To encapsulate creation logic and dependencies
```rust
// Example structure:
trait CheckerFactory {
    fn create(&self, config: &CheckConfig) -> Box<dyn Checker>;
}

// or simple factory function:
pub fn create_checker(check_type: CheckType) -> Result<Box<dyn Checker>, Error> {
    match check_type {
        CheckType::Ping => Ok(Box::new(PingChecker::new())),
        CheckType::Http => Ok(Box::new(HttpChecker::new())),
        // ...
    }
}
```

### Newtype Pattern
Use the Newtype pattern to provide type safety and semantic meaning:
- Wrap primitive types to prevent mixing incompatible values
- Add domain-specific methods and validation
- Example: AgentId(Uuid), Timestamp(i64), ResponseTime(Duration)

### RAII (Resource Acquisition Is Initialization)
Leverage Rust's ownership system for resource management:
- Acquire resources in constructors, release in Drop implementations
- Use guard types to ensure cleanup (e.g., MutexGuard, temporary files)
- Make resource lifetime explicit through type signatures

### Guidelines for Pattern Usage
- **Don't over-engineer**: Use patterns only when they provide clear value
- **Start simple**: Begin with straightforward implementations, refactor to patterns when complexity grows
- **Document patterns**: When using a pattern, add a comment explaining why it was chosen
- **Consistency**: Use the same pattern for similar problems throughout the codebase
- **Ergonomics**: Prioritize API usability and developer experience when applying patterns

Tracing should be implemented using the "tracing" crate with support for different log levels and output formats. 

# Testing Requirements

**CRITICAL**: For every piece of code generated, unit tests and integration tests MUST be created.

## Unit Tests
Unit tests should be placed in the same file as the code being tested, in a `#[cfg(test)]` module at the bottom of the file.

Requirements for unit tests:
- **Coverage**: Every public function, method, and struct must have corresponding unit tests
- **Edge Cases**: Test edge cases, error conditions, and boundary values
- **Mocking**: Use appropriate mocking techniques (e.g., `mockall` crate) for external dependencies
- **Async Testing**: Use `#[tokio::test]` for async functions
- **Assertions**: Use clear, descriptive assertions with helpful failure messages
- **Test Organization**: Group related tests using nested modules within the test module
- **Documentation**: Add doc comments to test functions explaining what is being tested and why

Example structure:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    mod function_name_tests {
        use super::*;
        
        #[test]
        fn test_normal_case() {
            // Test implementation
        }
        
        #[test]
        fn test_edge_case() {
            // Test implementation
        }
        
        #[test]
        fn test_error_condition() {
            // Test implementation
        }
    }
    
    #[tokio::test]
    async fn test_async_function() {
        // Async test implementation
    }
}
```

## Integration Tests
Integration tests should be placed in the `tests/` directory at the project root. Each integration test file tests a complete feature or workflow.

Requirements for integration tests:
- **Real Scenarios**: Test real-world usage scenarios and workflows
- **End-to-End**: Test complete feature flows from start to finish
- **File Naming**: Use descriptive names like `heartbeat_integration_tests.rs`, `config_loading_tests.rs`
- **Setup/Teardown**: Include proper setup and cleanup in tests
- **External Dependencies**: Mock external services (servers, databases) appropriately
- **Async Runtime**: Use tokio runtime for async integration tests
- **Timeouts**: Include reasonable timeouts for long-running tests
- **Isolation**: Tests should be isolated and not depend on each other

Example structure:
```rust
// tests/feature_integration_tests.rs
use agent_library::*;

#[tokio::test]
async fn test_complete_workflow() {
    // Setup
    let config = setup_test_config();
    
    // Execute
    let result = run_complete_workflow(config).await;
    
    // Assert
    assert!(result.is_ok());
    
    // Cleanup
    cleanup_test_resources();
}
```

## Test Data and Fixtures
- Use the `tests/fixtures/` directory for test data files (configs, sample data, etc.)
- Create helper functions for common test setup in `tests/common/mod.rs`
- Use builders or factory patterns for creating test objects

## Testing Tools and Crates
- **tokio**: For async testing with `#[tokio::test]`
- **mockall**: For creating mock objects
- **proptest** or **quickcheck**: For property-based testing (when appropriate)
- **rstest**: For parameterized tests
- **assert_matches**: For pattern matching in assertions
- **tempfile**: For temporary file/directory creation in tests

## When Generating Code
1. **Write the implementation code**
2. **Immediately write unit tests** in the same file
3. **Create or update integration tests** in the `tests/` directory
4. **Run tests** to verify they pass
5. **Document any test assumptions or requirements**

## Test Coverage Goals
- **Minimum**: 80% code coverage for all modules
- **Critical Code**: 95%+ coverage for core functionality (monitoring, reporting, configuration)
- **Error Paths**: All error paths and edge cases must be tested

## Example Test Scenarios to Always Include
- ✅ Normal/happy path
- ✅ Invalid input handling
- ✅ Null/empty values
- ✅ Boundary conditions (min/max values)
- ✅ Concurrent access (for shared state)
- ✅ Resource exhaustion scenarios
- ✅ Network failures (for I/O operations)
- ✅ Timeout scenarios
- ✅ Configuration errors
- ✅ State transitions


# Check types

MonitoringResult is an enum representing different types of monitoring checks that can be performed by the agent. Each variant of the enum corresponds to a specific type of check, such as Ping, Traceroute, TCP Connect, UDP Connect, HTTP GET, and Plugin-based checks.
It's helper methods:
- is_successful(): Determines success based on the specific check type
- response_time_ms(): Gets primary response time from specific result
- error_message(): Gets primary error message from specific result

Each variant contains detailed result structs:
- Ping(PingResult)
- Traceroute(TracerouteResult)
- TcpConnect(TcpConnectResult)
- UdpConnect(UdpConnectResult)
- HttpGet(HttpGetResult)
- Plugin(PluginResult)

Created detailed result structures:
- PingResult: Contains successes, failures, success_latencies, errors, avg_response_time_ms, resolved_ip
- TracerouteResult: Contains hops, target_reached, total_time_ms, errors
- TcpConnectResult: Contains connected, connect_time_ms, error, resolved_ip
- UdpConnectResult: Contains probe_successful, response_time_ms, error, resolved_ip
- HttpGetResult: Contains status_code, response_time_ms, response_size_bytes, error, success
- PluginResult: Contains plugin_name, plugin_version, success, response_time_ms, error, data

# Heartbeat System

The agent implements a lightweight heartbeat system for reporting agent health status to the server:
- AgentHeartbeat: Contains agent_id, timestamp, status, system metrics (CPU usage, memory usage, uptime)
- AgentHealthStatus: Enum with states (Healthy, Degraded, Critical, Unknown)
- HeartbeatReporter: Sends periodic health updates to server using system metrics from sysinfo crate
- Separate from full monitoring results for efficient status tracking

# Configuration Versioning and Server Polling

The agent configuration includes a `version` field (unsigned integer) that tracks the current configuration version. This is used for synchronization with the central server.

## Config Version Implementation
- Config struct has a `version` field (type `u32`) that starts at 1
- Version is included in configuration files (config.toml, config.example.toml)

## Future Implementation: Config Polling from Server
**TODO**: Implement periodic configuration polling from the server with the following behavior:
- Agent should periodically poll the server for updated configuration
- When polling, include current config version in the `X-Config-Version` HTTP header
- Server should compare the header version with the latest available config version
- If versions match (server version == current version), no update needed - server returns 304 Not Modified
- If server version is newer, server returns the new configuration with updated version number
- Agent should validate received config and update its running configuration
- Agent should persist the new config to disk for use after restarts
- On config update, agent should reload monitoring tasks with new endpoints and settings
- Config polling should respect server connectivity status (skip if server unreachable)
- Consider implementing exponential backoff for config polling failures


# Documentation

**IMPORTANT**: All project documentation MUST be maintained in the `docs/` folder. Do not create documentation files in the project root or other directories unless specifically required (e.g., README.md in root).

## Documentation Location Requirements
- **Primary Location**: All documentation files should be placed in the `docs/` directory
- **Root README**: No `README.md` in the project root - the main README should be in `docs/README.md`
- **Subdirectories**: Use subdirectories within `docs/` for different documentation categories if needed (e.g., `docs/api/`, `docs/guides/`, `docs/reference/`)
- **Cross-References**: Link between documentation files using relative paths from the `docs/` directory (e.g., `[API Reference](reference/api.md)`)
- **API Docs**: OpenAPI specifications belong in the `api/openapi/` directory, can be referenced from docs/ (e.g., `[OpenAPI Spec](../api/openapi/api/spec.yaml)`)
- **Organized Structure**: Use subdirectories within `docs/` for different documentation categories if needed
- **Cross-References**: Link between documentation files using relative paths from the `docs/` directory
- **API Docs**: OpenAPI specifications belong in the `api/openapi/` directory, can be referenced from docs/

## Required Documentation
The project should include comprehensive documentation covering:
- Project overview and architecture
- Installation instructions (from source and using Docker)
- Configuration options and examples
- Usage instructions for the agent and CLI
- Plugin development guide
- API documentation for the agent library
- OpenAPI specification for server API
- Contribution guidelines for developers
- Testing guidelines and requirements
- Troubleshooting and FAQ section

Documentation should be maintained in the `docs/` directory and linked from the main `README.md`. The `README.md` should provide a high-level overview and quick start guide, while detailed documentation can be organized in separate files within the `docs/` directory for better readability and maintenance.

## Feature Documentation
**CRITICAL**: All features implemented in the project MUST be documented in the `docs/` folder with practical examples.

### Documentation Requirements for Features
- **Create Dedicated Documentation**: For each major feature or module, create a corresponding documentation file in `docs/features/` (e.g., `docs/features/CLAIMING_IMPLEMENTATION.md`, `docs/features/HEARTBEAT_SYSTEM.md`)
- **Include Examples**: Every feature documentation MUST include practical, runnable examples showing how to use the feature
- **Code Snippets**: Provide clear code snippets demonstrating typical usage patterns
- **Configuration Examples**: Show relevant configuration options and their effects
- **Integration Examples**: Demonstrate how the feature integrates with other parts of the system
- **Edge Cases**: Document edge cases, limitations, and common pitfalls
- **API Examples**: For library features, show complete API usage examples including imports, setup, and cleanup
- **Command-Line Examples**: For CLI features, provide example commands with expected output

### Example Documentation Structure
```markdown
# Feature Name

## Overview
Brief description of what the feature does and why it exists.

## Usage

### Basic Example
```rust
// Complete, runnable code example
use agent_library::feature;

fn main() {
    // Example code here
}
```

### Configuration
```toml
# Example configuration
[feature]
option = "value"
```

### Advanced Usage
// More complex examples, edge cases, integration scenarios

## API Reference
// Key types, functions, and their purposes

## Common Pitfalls
// Things to watch out for
```

### Guidelines
- **Update on Change**: When modifying a feature, update its documentation and examples simultaneously
- **Test Examples**: Ensure all code examples in documentation compile and run correctly
- **Keep Current**: Documentation examples should reflect the current API and best practices
- **Link from Code**: Add doc comments in code referencing the detailed documentation in `docs/`

### Example Files
Runnable example code demonstrating library features should be placed in the `examples/` directory:

- **Location**: All example Rust files must be placed in the `examples/` directory at the project root
- **Naming Convention**: Use the format `XXX_name.rs` where:
  - `XXX` is a three-digit order number (e.g., `001`, `002`, `010`)
  - `name` is a descriptive name for the example (e.g., `basic_usage`, `heartbeat_demo`, `plugin_example`)
  - Examples: `001_basic_usage.rs`, `002_heartbeat_demo.rs`, `010_plugin_example.rs`
- **Content**: Each example should be a complete, runnable program demonstrating a specific feature or use case
- **Documentation**: Include comments in the example file explaining what it demonstrates
- **Reference**: Link to examples from feature documentation in `docs/`

## Project Structure
For detailed project structure, please refer to the [Project Structure](../docs/PROJECT_STRUCTURE.md) document.

## OpenAPI Specification and Type Generation

### OpenAPI Specification Location
The OpenAPI specification is maintained in `api/openapi/api/spec.yaml`. This specification defines the API contract between the agent and the central server.

### Type Generation with OMG

**CRITICAL**: All types related to the OpenAPI API model MUST be generated from the specification using the OMG (OpenAPI Model Generator) tool. Do NOT manually create types that are defined in the OpenAPI spec.

#### Generated Code Location
- **Specification**: `./api/openapi/api/spec.yaml`
- **Generated Types**: `./src/openapi/omg/generated/` module
- **Manual Extensions**: `./src/openapi/omg/responses.rs` (for reusable response components)
- **Module Entry**: Types are re-exported through `./src/openapi/mod.rs`

#### Using Generated Types

When referencing OpenAPI-generated types in code:

```rust
// Import from the openapi module
use crate::openapi;

// Use with openapi prefix for clarity
let registration = openapi::AgentRegistration {
    agent_id: agent_id.to_string(),
    claim_token_hash: hashed_token,
};

let response: openapi::ClaimAgentResponse201 = ...
```

**Guidelines for OpenAPI Types**:
- **Always use the `openapi` prefix**: This makes it immediately clear that a type comes from the API specification
- **Never create manual types**: If a type is in the OpenAPI spec, use the generated version
- **Import pattern**: Use `use crate::openapi;` and reference types as `openapi::TypeName`
- **Readability**: The `openapi::` prefix improves code readability by distinguishing API types from domain types
- **Documentation**: When the OpenAPI spec defines a type, that becomes the source of truth

#### When to Regenerate Types

Regenerate types after:
1. Modifying the OpenAPI specification (`api/openapi/api/spec.yaml`)
2. Adding new endpoints or response types
3. Changing existing schema definitions
4. Updating request/response structures

Use the provided regeneration script:
```bash
just generate-omg
```

Or manually:
```bash
omg --input ./api/openapi/api/spec.yaml -o ./src/openapi/omg/generated/
echo "pub mod responses;" >> ./src/openapi/omg/mod.rs
```

#### Manual Response Wrappers

Some response types must be manually maintained in `responses.rs` because OMG doesn't generate wrappers for reusable response components (`$ref: '#/components/responses/...'`). See [OPENAPI_CODE_GENERATION.md](../docs/features/OPENAPI_CODE_GENERATION.md) for details.

#### Example Workflow

1. **Define the API** in `api/openapi/api/spec.yaml`:
   ```yaml
   components:
     schemas:
       AgentHeartbeat:
         type: object
         properties:
           agent_id:
             type: string
           timestamp:
             type: integer
   ```

2. **Regenerate types**:
   ```bash
   just generate-omg
   ```

3. **Use in code**:
   ```rust
   use crate::openapi;
   
   let heartbeat = openapi::AgentHeartbeat {
       agent_id: self.agent_id.clone(),
       timestamp: Utc::now().timestamp(),
       // ... other fields
   };
   ```

4. **Never create parallel types**:
   ```rust
   // ❌ WRONG - Don't create this if it's in the OpenAPI spec
   struct AgentHeartbeat {
       agent_id: String,
       timestamp: i64,
   }
   
   // ✅ CORRECT - Use the generated type
   use crate::openapi;
   let heartbeat = openapi::AgentHeartbeat { ... };
   ```

For comprehensive information about OpenAPI type generation, regeneration workflows, and manual response wrappers, see [OPENAPI_CODE_GENERATION.md](../docs/features/OPENAPI_CODE_GENERATION.md).
