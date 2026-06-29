# Project description

This project is a distributed monitoring system designed to track reachability and performance of agents installed on various hosts. It consists of a central server that collects data from multiple agents deployed across different machines. The system provides real-time monitoring, alerting, and reporting capabilities to ensure the health and performance of the monitored infrastructure.

# Key Features
- **Agent-Based Monitoring**: Lightweight agents installed on hosts to collect metrics and send them to the central server.
- **Centralized Data Collection**: A server that aggregates data from all agents for analysis and reporting.
- **Real-Time Alerts**: Configurable alerts based on predefined thresholds to notify administrators of potential issues.
- **Performance Metrics**: Collection of various performance metrics such as reachability, response time and potentially other system metrics that can be extended via plugins.
- **Scalability**: Designed to handle a large number of agents and hosts efficiently.
- **Extensible Architecture**: Support for plugins to extend monitoring capabilities and integrate with other systems.
- **Config Hot-Reload**: Runtime configuration updates via file-system watch and SIGHUP signal, without restarting the agent.
- **Self-Upgrade**: Background update checker that downloads and replaces the binary from GitHub Releases.
- **User-Friendly Interface**: A web-based dashboard for visualizing data, configuring agents, and managing alerts.
- **APIs for Integration**: RESTful APIs to allow integration with other systems and automation tools.

# Technologies Used
- Agent Development is in Rust for performance and safety.
- Server-side components are developed in Go.
- Data storage using a time-series database (PostgreSQL + TimescaleDB) for efficient metric storage and retrieval.
- Web interface built with standard web technologies (HTML, CSS, TypeScript) for a responsive user experience CSS framework (e.g. Bulma).
- Communication between agents and server using RESTful APIs over HTTP/HTTPS.
- Containerization using Docker for easy deployment and scalability.
- Orchestration using Kubernetes for managing deployments in a clustered environment.
- Monitoring and logging using Prometheus and Grafana for system health and performance visualization.

# Instruction Files

Detailed, targeted instructions are in `.github/instructions/`:

| File | Applies to | Topic |
|---|---|---|
| `rust-code-style.instructions.md` | `**/*.rs` | Module structure, visibility, Rust patterns, tracing |
| `testing.instructions.md` | `**/*.rs`, `tests/**` | Unit & integration test requirements, workflows |
| `agent-domain.instructions.md` | `src/**` | Agent capabilities, check types, heartbeat, config versioning |
| `openapi-codegen.instructions.md` | `src/openapi/**`, `api/openapi/**` | OpenAPI type generation with OMG tool |
| `documentation.instructions.md` | `docs/**`, `examples/**`, `**/*.md` | Documentation structure, feature docs, example files |

