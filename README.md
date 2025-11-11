# Smotra Agent

A distributed monitoring agent for tracking reachability and performance of networked hosts.

## Features

- **ICMP Ping Monitoring**: Check host reachability using ICMP echo requests
- **Configurable Intervals**: Set custom monitoring intervals and timeouts
- **Concurrent Checks**: Perform multiple checks simultaneously with configurable limits
- **Central Reporting**: Send monitoring data to a central server
- **Offline Operation**: Cache results locally when server is unreachable
- **Plugin System**: Extend functionality with custom monitoring plugins
- **Interactive TUI**: Monitor status with an interactive terminal interface
- **Low Resource Usage**: Built with Rust and async/await for efficiency

## Architecture

The agent is implemented as a library (`smotra_agent`) with multiple binary tools:

- `agent`: Main daemon for running the monitoring agent
- `agent-cli`: Interactive TUI for monitoring and configuration
- `agent-plugin-example`: Example plugin demonstrating extensibility
- `agent-updater`: Auto-update tool (coming soon)

## Installation

### From Source

```bash
cargo build --release
```

Binaries will be available in `target/release/`:
- `agent`
- `agent-cli`
- `agent-plugin-example`
- `agent-updater`

## Configuration

Generate a default configuration file:

```bash
./agent --gen-config
```

Or use the CLI:

```bash
./agent-cli gen-config -o config.toml
```

Example configuration:

```toml
agent_id = "unique-agent-id"
tags = ["production", "web-servers"]

[monitoring]
interval_secs = 60
timeout_secs = 5
ping_count = 3
max_concurrent = 10
traceroute_on_failure = false
traceroute_max_hops = 30

[server]
url = "https://monitoring.example.com"
api_key = "your-api-key"
report_interval_secs = 300
verify_tls = true
timeout_secs = 30
retry_attempts = 3

[storage]
cache_dir = "./cache"
max_cached_results = 10000
max_cache_age_secs = 86400

[[endpoints]]
address = "8.8.8.8"
tags = ["dns", "google"]

[[endpoints]]
address = "example.com"
port = 443
tags = ["web"]
```

## Usage

### Running the Agent

Start the monitoring agent:

```bash
./agent -c config.toml
```

With custom log level:

```bash
./agent -c config.toml --log-level debug
```

### Using the CLI

Interactive TUI:

```bash
./agent-cli -c config.toml tui
```

Show current status:

```bash
./agent-cli -c config.toml status
```

Validate configuration:

```bash
./agent-cli -c config.toml validate-config
```

### TUI Controls

- **Arrow Keys / h/l**: Navigate between tabs
- **s**: Start monitoring
- **q / Esc**: Quit
- **Ctrl+C**: Force quit

## Library Usage

The agent can also be embedded in other Rust applications:

```rust
use smotra_agent::{Agent, Config, Endpoint};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration
    let mut config = Config::default();
    config.agent_id = "my-agent".to_string();
    
    // Add endpoints
    config.endpoints.push(Endpoint::new("8.8.8.8").with_tags(vec!["dns".to_string()]));
    
    // Create and start agent
    let agent = Agent::new(config);
    agent.start().await?;
    
    Ok(())
}
```

## Plugin Development

Create custom monitoring plugins by implementing the `MonitoringPlugin` trait:

```rust
use async_trait::async_trait;
use smotra_agent::{
    plugin::MonitoringPlugin,
    types::{Endpoint, MonitoringResult},
};

struct MyPlugin;

#[async_trait]
impl MonitoringPlugin for MyPlugin {
    fn name(&self) -> &str {
        "my_plugin"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    async fn check(&self, agent_id: &str, endpoint: &Endpoint) 
        -> smotra_agent::error::Result<MonitoringResult> 
    {
        // Your monitoring logic here
        todo!()
    }
}
```

See `agent-plugin-example` for a complete example.

## Development

### Prerequisites

- Rust 1.70 or later
- Linux, macOS, or Windows
- Administrator/root privileges for ICMP operations

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running with Debug Logging

```bash
RUST_LOG=debug cargo run --bin agent -- -c config.toml
```

## License

MIT

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.
