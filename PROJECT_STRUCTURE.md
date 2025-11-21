# Smotra Agent - Project Structure

## Generated Files

### Core Library Files
- `src/lib.rs` - Main library entry point with Agent struct and public exports
- `src/error.rs` - Error types and Result aliases

### Configuration Module (`src/config/`)
- `src/config/mod.rs` - Module exports for configuration
- `src/config/loader.rs` - Configuration loading and validation logic
- `src/config/types.rs` - Configuration data structures (Config, MonitoringConfig, ServerConfig, etc.)

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

### Binaries (`src/bin/`)
- `src/bin/agent.rs` - Main daemon for running the agent
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
- `examples/plugin.rs` - HTTP monitoring plugin example
- `examples/heartbeat_demo.rs` - Heartbeat demonstration example


### Tests (`tests/`)
- `tests/heartbeat_integration_tests.rs` - Integration tests for heartbeat functionality

### Configuration & Documentation
- `Cargo.toml` - Rust project configuration with dependencies
- `README.md` - Comprehensive documentation
- `config.example.toml` - Example configuration file
- `config.toml` - Local configuration file
- `PROJECT_STRUCTURE.md` - This file
- `.gitignore` - Git ignore patterns

## Key Features Implemented

### Agent Library (`src/lib.rs`)
- Agent struct for coordinating all operations
- Start/stop methods for lifecycle management
- Status tracking and configuration updates
- Graceful shutdown handling

### Configuration System
- TOML-based configuration
- Validation logic
- Default values for all settings
- Support for multiple endpoints with tags
- Server connection configuration
- Local storage settings

### Monitoring System
- ICMP ping checks using surge-ping
- Concurrent checking with semaphore limits
- Configurable intervals and timeouts
- Multiple pings per check with average calculation
- DNS resolution support
- Result channel for processing

### Reporting System
- Periodic reporting to central server
- HTTP client with authentication
- Connection status tracking
- Cache manager structure (to be implemented)
- Heartbeat reporting with system metrics (CPU, memory, uptime)
- Agent health status monitoring

### Plugin System
- MonitoringPlugin trait for custom checks
- Plugin registry for managing plugins
- Example HTTP plugin implementation

### CLI Tools
- `agent`: Full-featured daemon with logging
- `agent-cli`: Interactive TUI with:
  - Status dashboard with real-time updates
  - Endpoints list view
  - Configuration viewer
  - Logs view (placeholder)
  - Tab navigation and keyboard controls

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
1. **Testing**: Add unit and integration tests
2. **Traceroute**: Implement traceroute functionality
3. **Cache Implementation**: Complete local caching to disk
4. **Result Processing**: Connect monitoring results to reporter

### Medium Priority
1. **TCP/UDP Checks**: Add TCP and UDP connectivity checks
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
./agent-cli tui -c config.toml
./agent-cli status -c config.toml
./agent-cli validate-config -c config.toml
```

### Run Plugin Example
```bash
./agent-plugin-example
```

## Building

```bash
cargo build --release
```

Binaries will be in `target/release/`:
- `agent`
- `agent-cli`
- `agent-plugin-example`
- `agent-updater`

## Notes

- ICMP operations require elevated privileges (root/administrator)
- The agent can run in standalone mode without server connection
- Results are cached locally when server is unreachable
- Plugin system allows for extensibility without modifying core
- TUI provides real-time monitoring without log file inspection
