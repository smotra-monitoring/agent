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
- `README.md`: Documentation for the project.
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

- mod.rs files should not functional code should. The only mod definitions or re-export can be present in there. 
- "utilities" module can be used inside other modules to declare only private support functions for the module.
- reserve "support" module for external functions that cna be used by other projects in cargo workspaces


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



## Project Structure
For detailed project structure, please refer to the [Project Structure](../PROJECT_STRUCTURE.md) document.

## OpenApi Specification
For detailed OpenAPI specification, please refer to the [OpenAPI Specification](../api/openapi/api/spec.yaml) document.
