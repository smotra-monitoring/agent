# Smotra Agent - Project Structure

## Generated Files

### Core Library Files
- `src/lib.rs` - Main library entry point with Agent struct
- `src/error.rs` - Error types and Result aliases
- `src/types.rs` - Common types (MonitoringResult, AgentStatus, Endpoint, etc.)

### Configuration
- `src/config/mod.rs` - Configuration loading and validation
- `src/config/types.rs` - Configuration data structures

### Monitoring
- `src/monitor/mod.rs` - Monitoring coordinator and task management
- `src/monitor/ping.rs` - ICMP ping implementation using surge-ping

### Reporting
- `src/reporter/mod.rs` - Data reporter with server communication and caching

### Plugin System
- `src/plugin/mod.rs` - Plugin trait and registry for extensibility

### Binaries
- `src/bin/agent.rs` - Main daemon for running the agent
- `src/bin/agent_cli.rs` - Interactive TUI with Ratatui
- `src/bin/agent_plugin_example.rs` - HTTP monitoring plugin example
- `src/bin/agent_updater.rs` - Auto-updater stub

### Configuration & Documentation
- `Cargo.toml` - Rust project configuration with dependencies
- `README.md` - Comprehensive documentation
- `config.example.toml` - Example configuration file
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
