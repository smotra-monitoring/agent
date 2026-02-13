# Smotra Agent

[![CLA](https://img.shields.io/badge/CLA-signed-brightgreen)](https://cla-assistant.io/smotra-monitoring/agent)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](../LICENSE)

A lightweight, distributed monitoring agent written in Rust for tracking reachability and performance of networked hosts. Part of the Smotra monitoring system.

## 🚀 Quick Start

### Using Quick Install Script (Coming Soon)

```bash
# One-line install (when available)
curl -fsSL https://install.smotra.net/agent.sh | sh
```

See [INSTALL_SCRIPT.md](INSTALL_SCRIPT.md) for implementation details and requirements.

### Building from Source

```bash
# Build from source
cargo build --release

# Generate configuration
./target/release/agent --gen-config

# Edit config.toml with your settings
# Then start the agent
./target/release/agent -c config.toml
```

## ✨ Key Features

- **ICMP Ping & Traceroute** - Check host reachability with detailed path analysis
- **Zero-Config Onboarding** - Self-registration with secure claim-based workflow
- **Heartbeat Reporting** - Automatic agent health status updates with system metrics
- **Offline-First** - Caches results locally when server is unreachable
- **Low Resource Usage** - Built with Rust async/await for efficiency
- **Plugin System** - Extend monitoring capabilities with custom plugins
- **Interactive TUI** - Real-time monitoring dashboard with `agent-cli`
- **Concurrent Checks** - Perform multiple checks simultaneously

### Coming Soon
- Traceroute analysis (types defined, implementation pending)
- TCP/UDP/HTTP connectivity checks (types defined, implementation pending)

## 📋 Requirements

- Rust 1.70 or later
- Administrator/root privileges for ICMP operations
- Linux, macOS, or Windows

## 🏗️ Architecture

The agent consists of:

- **`smotra_agent`** - Core monitoring library
- **`agent`** - Main daemon process
- **`agent-cli`** - Interactive TUI for monitoring and configuration
- **`agent-updater`** - Auto-update tool (stub)

## 🔧 Basic Configuration

```toml
version = 1
agent_id = "00000000-0000-0000-0000-000000000000" # Will be set automatically after claiming
agent_name = "Production Agent"
tags = ["production", "datacenter-1"]

[monitoring]
interval_secs = 60
timeout_secs = 5
ping_count = 3

[server]
url = "https://api.smotra.net"
# api_key will be set automatically after claiming
report_interval_secs = 300
heartbeat_interval_secs = 300

[[endpoints]]
id = "01931ab4-b278-7f64-a32f-dae3cabe1ff0"
address = "8.8.8.8"
tags = ["dns"]
enabled = true
```

## 📖 Documentation

- **[Complete User Guide](GUIDE.md)** - Detailed installation, configuration, and usage
- **[Project Structure](PROJECT_STRUCTURE.md)** - Code organization and architecture
- **[Implementation Status](IMPLEMENTATION_STATUS.md)** - Current implementation status and roadmap
- **[Claiming Implementation](CLAIMING_IMPLEMENTATION.md)** - Agent registration details
- **[Development Roadmap](ROADMAP.md)** - Development plans and upcoming features

## 🔐 Agent Registration & Claiming

The agent supports zero-configuration onboarding:

1. Configure server URL in `config.toml`
2. Start the agent - it displays a claim token
3. Admin claims the agent via web interface
4. Agent automatically receives API key and starts monitoring

See the [User Guide](GUIDE.md#agent-self-registration-and-claiming) for complete details.

## 🧩 Library Usage

Embed the agent in your Rust applications:

```rust
use smotra_agent::{Agent, Config, Endpoint};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::default();
    config.agent_id = "00000000-0000-0000-0000-000000000000".to_string();
    config.endpoints.push(
        Endpoint::new("8.8.8.8").with_tags(vec!["dns".to_string()])
    );
    
    let agent = Agent::new(config);
    agent.start().await?;
    Ok(())
}
```

## 🔌 Plugin Development

Create custom monitoring plugins - see [examples/010_plugin.rs](../examples/010_plugin.rs):

```rust
use async_trait::async_trait;
use smotra_agent::plugin::MonitoringPlugin;

struct MyPlugin;

#[async_trait]
impl MonitoringPlugin for MyPlugin {
    fn name(&self) -> &str { "my_plugin" }
    fn version(&self) -> &str { "0.1.0" }
    
    async fn check(&self, agent_id: &str, endpoint: &Endpoint) 
        -> smotra_agent::error::Result<MonitoringResult> 
    {
        // Your monitoring logic here
        todo!()
    }
}
```

## 🧪 Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run --bin agent -- -c config.toml
```

## 📦 API Specification

OpenAPI specification will be available at `api/openapi/` (currently in development).

## 📄 License

Apache 2.0 License - see [LICENSE](../LICENSE) for details.

## 🤝 Contributing
This is an open-source project, and we deeply value the community's interest. However, due to the high volume of automated and machine-generated code currently being submitted, we are significantly narrowing our intake process to maintain code quality and project stability.

### Current PR Policy
To ensure our maintainers can focus on meaningful improvements:

- **General Pull Requests:** We are currently not accepting unsolicited PRs, particularly those containing generated code.

- **Exceptions:** Critical bug fixes or major feature enhancements are still considered on a case-by-case basis.

- **CLA Requirement:** If an exception is made for your contribution, you must sign our Contributor License Agreement (CLA) before any code can be reviewed or merged.

### How to Help
If you’ve found a bug or have a great idea:

- **Open an Issue:** Please start by searching existing issues or creating a new one to discuss your proposal.

- **Wait for Feedback:** Do not spend time on a PR until a maintainer has greenlit the approach in the issue comments.

- **Follow the Workflow:** Once approved, you may fork the repo, work in a feature branch, and submit your PR (with tests!) for final review.

See the [User Guide](GUIDE.md) for testing requirements and code standards.

---

**Need help?** See the [Complete User Guide](GUIDE.md) for detailed documentation.
