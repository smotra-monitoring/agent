# Smotra Agent - Project Structure

## Generated Files

### Core Library Files
- `src/lib.rs` - Main library entry point with Agent struct and public exports
- `src/error.rs` - Error types and Result aliases

### Configuration Module (`src/agent_config/`)
- `src/agent_config/mod.rs` - Module exports for configuration
- `src/agent_config/loader.rs` - Configuration loading, validation, and secure saving logic
- `src/agent_config/types.rs` - Configuration data structures (Config, MonitoringConfig, StorageConfig)
- `src/agent_config/server_config/` - Server configuration submodule
  - `src/agent_config/server_config/mod.rs` - Server config module exports
  - `src/agent_config/server_config/types.rs` - ServerConfig and ClaimConfig structures

### Core Module (`src/core/`)
- `src/core/mod.rs` - Module exports for core types
- `src/core/agent.rs` - Agent struct implementation with lifecycle management
- `src/core/types.rs` - Core types (MonitoringResult, AgentStatus, Endpoint, CheckType, etc.)

### Monitoring Module (`src/monitor/`)
- `src/monitor/mod.rs` - Module exports for monitoring
- `src/monitor/coordinator.rs` - Monitoring coordinator and task management
- `src/monitor/ping.rs` - ICMP ping implementation using surge-ping (PingChecker)

### Reporter Module (`src/reporter/`)
- `src/reporter/mod.rs` - Module exports for reporting
- `src/reporter/server.rs` - Server communication and result reporting
- `src/reporter/cache.rs` - Cache manager for offline data storage
- `src/reporter/heartbeat.rs` - Heartbeat reporter for lightweight agent status updates

### Plugin System (`src/plugin/`)
- `src/plugin/mod.rs` - Module exports for plugin system
- `src/plugin/trait_def.rs` - MonitoringPlugin trait definition
- `src/plugin/registry.rs` - Plugin registry for managing plugins

### Claiming Workflow Module (`src/claim/`)
- `src/claim/mod.rs` - Module exports for claiming workflow
- `src/claim/types.rs` - Data structures for registration and claim status (AgentRegistration, ClaimResult, ClaimStatus)
- `src/claim/token.rs` - Claim token generation and hashing (SHA-256)
- `src/claim/registration.rs` - Agent self-registration with retry logic
- `src/claim/polling.rs` - Claim status polling with configurable interval
- `src/claim/display.rs` - User-friendly claim information display
- `src/claim/workflow.rs` - Main Claim orchestrator struct coordinating the complete workflow

### Configuration Persistence (`src/agent_config/loader.rs`)
- Secure API key storage with file permissions (0600 on Unix)
- Configuration file updates with TOML serialization
- Atomic file writes to prevent corruption
- Agent ID persistence via `apply_claim_result()` method
- Async save operations with `save_to_file_secure()`

### Binaries (`src/bin/`)
- `src/bin/agent.rs` - Main daemon with integrated claiming workflow
- `src/bin/agent_updater.rs` - Auto-updater stub

### Binary - Interactive agent TUI with Ratatui
- `src/bin/agent_cli/main.rs` - Main entry point
- `src/bin/agent_cli/cli_args.rs` - Command-line argument definitions
- `src/bin/agent_cli/commands.rs` - Status, validation, and config generation handlers
- `src/bin/agent_cli/logging.rs` - Log buffer and initialization logic
- `src/bin/agent_cli/tui/mod.rs` - TUI initialization
- `src/bin/agent_cli/tui/render.rs` - TUI-related functionality rendering functions
- `src/bin/agent_cli/tui/ui_loop.rs` - TUI-related functionality event loop
- `src/bin/agent_cli/tui/runner.rs` - TUI-related functionality initializing and starting event loop

### Examples (`examples/`)
- `examples/010_plugin.rs` - HTTP monitoring plugin example
- `examples/002_heartbeat_demo.rs` - Heartbeat demonstration example


### Tests (`tests/`)
- `tests/heartbeat_integration_tests.rs` - Integration tests for heartbeat functionality
- `tests/claim_integration_tests.rs` - Integration tests for claiming workflow with mock server

### OpenAPI Module (`src/openapi/`)
- `src/openapi/mod.rs` - Module entry point, re-exports types from omg
- `src/openapi/omg/mod.rs` - OMG-generated module with manual response extensions
- `src/openapi/omg/generated/` - Auto-generated types from OpenAPI spec (DO NOT EDIT MANUALLY)
  - `src/openapi/omg/generated/mod.rs` - Generated module structure
  - `src/openapi/omg/generated/models.rs` - Generated schema and response types
- `src/openapi/omg/responses.rs` - Manually maintained response wrappers for reusable components

**Important**: All API model types are generated from the OpenAPI specification at `api/openapi/api/spec.yaml` using the OMG tool. Never manually create types that are defined in the spec. Use `use crate::openapi;` and reference types with the `openapi::` prefix for clarity.

### Configuration & Documentation
- `Cargo.toml` - Rust project configuration with dependencies
- `README.md` - Comprehensive documentation
- `config.example.toml` - Example configuration file
- `PROJECT_STRUCTURE.md` - This file
- `.gitignore` - Git ignore patterns
- `api/openapi/api/spec.yaml` - OpenAPI specification defining the server API
- `docs/OPENAPI_CODE_GENERATION.md` - Documentation for type generation workflow

## Key Features Implemented

### Agent Library (`src/lib.rs`)
- Agent struct for coordinating all operations
- Start/stop methods for lifecycle management
- Status tracking and configuration updates
- Graceful shutdown handling

### Configuration System
- TOML-based configuration
- Configuration versioning (version field) for server synchronization
- Validation logic
- Default values for all settings
- Support for multiple endpoints with UUIDs, tags, and enabled flag
- Server connection configuration with heartbeat intervals
- Local storage settings for caching
- Claiming workflow configuration (poll interval, max retries)

### Monitoring System
- **ICMP Ping** (✅ Implemented): surge-ping based ping checks with concurrent execution
- **Traceroute** (⏳ Pending): Types defined (TracerouteResult, TracerouteHop) but checker not yet implemented
- **TCP Connect** (⏳ Pending): Types defined (TcpConnectResult) but checker not yet implemented
- **UDP Connect** (⏳ Pending): Types defined (UdpConnectResult) but checker not yet implemented
- **HTTP GET** (⏳ Pending): Types defined (HttpGetResult) but checker not yet implemented
- Concurrent checking with semaphore limits
- Configurable intervals and timeouts
- Multiple pings per check with average calculation
- DNS resolution support
- Result channel for processing

### Reporting System
- Periodic reporting to central server
- HTTP client with X-API-KEY header authentication (reqwest with rustls-tls)
- Connection status tracking
- Cache manager stub (disk persistence to be implemented)
- Heartbeat reporting with system metrics (CPU, memory, uptime) using sysinfo crate
- Agent health status monitoring (Healthy, Degraded, Critical, Unknown)

### Agent Self-Registration and Claiming Workflow
- **Zero-configuration onboarding**: Agents can self-register without manual API key distribution
- **Secure token generation**: Cryptographically secure 64-character claim tokens
- **Token hashing**: SHA-256 hashing before network transmission (defense in depth)
- **Agent self-registration**: POST to `/api/v1/agent/register` with agent ID and token hash
- **Polling mechanism**: Periodic status checks with configurable interval
- **API key delivery**: One-time delivery when administrator claims agent
- **Secure persistence**: API key saved with 0600 permissions on Unix systems
- **Retry logic**: Exponential backoff for registration failures
- **User-friendly display**: Formatted claim information box with instructions
- **Idempotent registration**: Multiple registration attempts don't create duplicates
- **Time-limited tokens**: Claims expire after 24 hours (server-side)

### Plugin System
- MonitoringPlugin trait for custom checks
- Plugin registry for managing plugins
- Example HTTP plugin implementation

### CLI Tools
- `agent`: Full-featured daemon with logging and signal handling
- `agent-cli`: Interactive TUI with:
  - Status dashboard with real-time updates
  - Endpoints list view with monitoring results
  - Configuration viewer
  - Logs view with buffered output
  - Tab navigation and keyboard controls (Arrow keys, h/l, s to start, q/Esc to quit)
  - Commands: `tui`, `status`, `validate-config`, `gen-config`
- `agent-updater`: Placeholder for auto-update functionality

## Architecture Highlights

### Async/Await Design
- Built on Tokio runtime for high performance
- Non-blocking I/O operations
- Concurrent task execution with limits

### Thread Safety
- RwLock for shared state (Config, AgentStatus)
- Broadcast channels for shutdown signaling
- MPSC channels for result streaming

### Error Handling
- Custom Error enum with thiserror
- Result type alias throughout
- Proper error propagation

### Logging & Tracing
- Structured logging with tracing crate
- Configurable log levels
- Thread and line number tracking

## Next Steps for Implementation

### High Priority
1. **Traceroute**: Implement TracerouteChecker (types already defined)
2. **TCP/UDP/HTTP Checks**: Implement TcpConnectChecker, UdpConnectChecker, HttpGetChecker (types already defined)
3. **Cache Implementation**: Complete local caching to disk (stub currently in place)
4. **Result Processing**: Connect monitoring results to reporter
5. **OpenAPI Specification**: Create complete API specification in api/openapi/

### Medium Priority
1. **Enhanced Testing**: Expand unit and integration test coverage
2. **Plugin Loading**: Dynamic plugin loading from shared libraries
3. **Auto-updates**: Implement update checking and installation
4. **Log Viewer**: Add real-time log viewing in TUI

### Low Priority
1. **Metrics**: Add Prometheus metrics endpoint
2. **Health Check**: HTTP endpoint for health status
3. **Configuration Hot-reload**: Support runtime config updates
4. **Advanced TUI**: Add graphs and more interactive features
5. **Retry in server reporting** Implement retry logic in reporter/server.rs
6. **Track number of failed sends to server** Add tracking to server status struct

## Usage Examples

### Generate Configuration
```bash
./agent --gen-config
# or
./agent-cli gen-config -o config.toml
```

### Run Agent
```bash
./agent -c config.toml
./agent -c config.toml --log-level debug
```

### Run Interactive CLI
```bash
./agent-cli -c config.toml tui
./agent-cli -c config.toml status
./agent-cli -c config.toml validate-config
```

### Run Plugin Examples
```bash
cargo run --example 010_plugin
cargo run --example 002_heartbeat_demo
```

## Building

```bash
cargo build --release
```

Binaries will be in `target/release/`:
- `agent`
- `agent-cli`
- `agent-updater`

Example plugins in `examples/`:
- `010_plugin` - HTTP monitoring plugin example
- `002_heartbeat_demo` - Heartbeat system demonstration

## Notes

- ICMP operations require elevated privileges (root/administrator)
- The agent can run in standalone mode without server connection
- Results are cached locally when server is unreachable
- Plugin system allows for extensibility without modifying core
- TUI provides real-time monitoring without log file inspection
