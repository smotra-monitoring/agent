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
- `src/monitor/server.rs` - Monitoring task coordination and execution loop
- `src/monitor/ping.rs` - ICMP ping implementation using surge-ping (PingChecker)

### Cache Module (`src/cache/`)
- `src/cache/mod.rs` - Module exports for cache
- `src/cache/store.rs` - `ResultCache` — in-memory cache for offline monitoring results, with disk persistence

### Reporter Module (`src/reporter/`)
- `src/reporter/mod.rs` - Module exports for reporting
- `src/reporter/server.rs` - Server communication and result reporting
- `src/reporter/results.rs` - Result batch submission to server
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
- `src/bin/smotra.rs` - Main daemon binary with integrated claiming workflow

### Binary - Interactive agent TUI with Ratatui (`src/bin/smotra_cli/`)
- `src/bin/smotra_cli/main.rs` - Main entry point
- `src/bin/smotra_cli/cli_args.rs` - Command-line argument definitions
- `src/bin/smotra_cli/commands.rs` - Status, validation, and config generation handlers
- `src/bin/smotra_cli/logging.rs` - Log buffer and initialization logic
- `src/bin/smotra_cli/tui/mod.rs` - TUI initialization
- `src/bin/smotra_cli/tui/render.rs` - TUI rendering functions
- `src/bin/smotra_cli/tui/ui_loop.rs` - TUI event loop
- `src/bin/smotra_cli/tui/runner.rs` - TUI initialization and event loop startup

### Self-Upgrade Module (`src/self_upgrade/`)
- `src/self_upgrade/mod.rs` - Module exports and public API
- `src/self_upgrade/checker.rs` - Version comparison and update detection
- `src/self_upgrade/downloader.rs` - Release binary download and checksum verification
- `src/self_upgrade/environment.rs` - Containerized runtime detection
- `src/self_upgrade/github.rs` - GitHub Releases API integration
- `src/self_upgrade/replacer.rs` - Binary replacement using `self-replace`
- `src/self_upgrade/server.rs` - Background update checker task

### Hot-Reload Module (`src/agent_config/hot_reload/`)
- `src/agent_config/hot_reload/mod.rs` - Module exports and orchestrator
- `src/agent_config/hot_reload/config_file_watcher.rs` - File-system watcher (`ConfigFileWatcher`)
- `src/agent_config/hot_reload/server.rs` - Hot-reload server coordinating watch, SIGHUP, and reload
- `src/agent_config/hot_reload/sighup.rs` - SIGHUP signal handler

### Examples (`examples/`)
- `examples/010_plugin.rs` - HTTP monitoring plugin example
- `examples/011_plugin_registry.rs` - Plugin registry usage example
- `examples/020_self_upgrade.rs` - Self-upgrade workflow example

### Tests (`tests/`)
- `tests/claim_api_test.rs` - Unit/API tests for claiming primitives
- `tests/claim_integration_tests.rs` - Integration tests for claiming workflow with mock server
- `tests/self_upgrade_integration_tests.rs` - Integration tests for self-upgrade (version check, download)

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
- Cache manager with disk persistence for offline data storage
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
- `smotra`: Full-featured daemon with logging, signal handling, and integrated claiming workflow
- `smotra-cli`: Interactive TUI with:
  - Status dashboard with real-time updates
  - Endpoints list view with monitoring results
  - Configuration viewer
  - Logs view with buffered output
  - Tab navigation and keyboard controls (Arrow keys, h/l, s to start, q/Esc to quit)
  - Commands: `tui`, `status`, `validate-config`, `gen-config`

### Self-Upgrade
- Background Tokio task checks GitHub Releases for newer versions
- Containerized environment detection disables auto-upgrade automatically
- Config-driven update URL and polling interval
- Binary replacement using `self-replace` crate with OS-specific restart flow

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
3. **Cache disk persistence**: Complete on-disk persistence in `src/cache/store.rs`
4. **Config server polling**: Implement server-side config polling with version tracking

### Medium Priority
1. **Enhanced Testing**: Expand unit and integration test coverage
2. **Plugin Loading**: Dynamic plugin loading from shared libraries
3. **Advanced TUI**: Add graphs and more interactive features
4. **Retry in server reporting**: Implement retry logic in `reporter/server.rs`

### Low Priority
1. **Metrics**: Add Prometheus metrics endpoint
2. **Health Check**: HTTP endpoint for health status
3. **Track failed sends**: Add tracking to server status struct

## Usage Examples

### Generate Configuration
```bash
./smotra --gen-config
# or
./smotra-cli gen-config -o config.toml
```

### Run Agent
```bash
./smotra -c config.toml
./smotra -c config.toml --log-level debug
```

### Run Interactive CLI
```bash
./smotra-cli -c config.toml tui
./smotra-cli -c config.toml status
./smotra-cli -c config.toml validate-config
```

### Run Plugin Examples
```bash
cargo run --example 010_plugin
cargo run --example 011_plugin_registry
cargo run --example 020_self_upgrade
```

## Building

```bash
cargo build --release
```

Binaries will be in `target/release/`:
- `smotra`
- `smotra-cli`

Example plugins in `examples/`:
- `010_plugin` - HTTP monitoring plugin example
- `011_plugin_registry` - Plugin registry usage example
- `020_self_upgrade` - Self-upgrade workflow example

## Notes

- ICMP operations require elevated privileges (root/administrator)
- The agent can run in standalone mode without server connection
- Results are cached locally when server is unreachable
- Plugin system allows for extensibility without modifying core
- TUI provides real-time monitoring without log file inspection
