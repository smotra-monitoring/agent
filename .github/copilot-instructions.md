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
- `agent-plugin-example`: Example plugin demonstrating how to extend agent functionality.
- `agent-updater`: Binary to handle auto-updates for the agent.

Files structure
- `src/`: Source code for the agent library and binaries.
- `Cargo.toml`: Cargo configuration file for the project.
- `Dockerfile`: Dockerfile for building the agent container image.
- `README.md`: Documentation for the project.
- `examples/`: Example configurations and usage scenarios.
- `plugins/`: Directory for agent plugins.
- `tests/`: Unit and integration tests for the agent library and binaries.
- `src/lib.rs`: Main library file for the agent.
- `src/bin/`: Directory containing the main binaries for the agent.
- `src/bin/agent.rs`: Main binary to run the monitoring agent.
- `src/bin/agent_cli.rs`: Command-line interface for managing and configuring the agent.
- `src/bin/agent_plugin_example.rs`: Example plugin demonstrating how to extend agent functionality.
- `src/bin/agent_updater.rs`: Binary to handle auto-updates for the agent

# Code organization

- mod.rs files should not functional code should. The only mod definitions or re-export can be present in there. 
- "utilities" module can be used inside other modules to declare only private support functions for the module.
- reserve "support" module for external functions that cna be used by other projects in cargo workspaces


Tracing should be implemented using the "tracing" crate with support for different log levels and output formats. 


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



## Project Structure
For detailed project structure, please refer to the [Project Structure](PROJECT_STRUCTURE.md) document.
