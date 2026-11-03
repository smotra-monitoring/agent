---
applyTo: "src/**"
---

# Agent Implementation Details

## Project Structure

- `src/` — Source code for the agent library and binaries.
- `Cargo.toml` — Cargo configuration file.
- `docs/` — Documentation for the project.
- `docs/features/` — Feature-specific documentation.
- `examples/` — Example plugin implementations.
- `tests/` — Integration tests.
- `api/` — OpenAPI specification and related documentation.
- `src/lib.rs` — Main library file for the agent.
- `src/bin/smotra.rs` — Main agent binary.
- `src/bin/smotra_cli/` — CLI with TUI implementation (Ratatui).
  - `main.rs`, `cli_args.rs`, `commands.rs`, `logging.rs`
  - `tui/` — TUI implementation (mod.rs, render.rs, runner.rs, ui_loop.rs)
- `src/self_upgrade/` — Self-upgrade module (background update checker, GitHub Releases integration, binary replacement).

## Agent Capabilities

- Operates in standalone mode if the server is unreachable (caches data locally, sends when connection restored).
- Supports auto-updates via the built-in `src/self_upgrade/` module (background Tokio task, GitHub Releases, `self-replace` crate).
- Uses ICMP ping (`surge-ping`) for reachability checks; traceroute types are defined but checker is not yet implemented; TCP/UDP/HTTP via plugins.
- Configuration includes: list of endpoints to monitor, monitoring intervals.
- Endpoints provided by server or configured locally. Agents may be grouped by tags and form meshes.
- Agent library is embeddable in other Rust applications via a simple API (start/stop tasks, configure endpoints, retrieve data).
- Use `tokio` async runtime throughout for low resource usage.

## Check Types (MonitoringResult)

`MonitoringResult` is an enum of all check variants. Helper methods:
- `is_successful()` — success based on check type
- `response_time_ms()` — primary response time
- `error_message()` — primary error message

Variants and their result structs:
- `Ping(PingResult)` — `successes`, `failures`, `success_latencies`, `errors`, `avg_response_time_ms`, `resolved_ip`
- `Traceroute(TracerouteResult)` — `hops`, `target_reached`, `total_time_ms`, `errors`
- `TcpConnect(TcpConnectResult)` — `connected`, `connect_time_ms`, `error`, `resolved_ip`
- `UdpConnect(UdpConnectResult)` — `probe_successful`, `response_time_ms`, `error`, `resolved_ip`
- `HttpGet(HttpGetResult)` — `status_code`, `response_time_ms`, `response_size_bytes`, `error`, `success`
- `Plugin(PluginResult)` — `plugin_name`, `plugin_version`, `success`, `response_time_ms`, `error`, `data`

## Heartbeat System

Lightweight heartbeat for reporting agent health to the server (separate from full monitoring results):
- `AgentHeartbeat` — `agent_id`, `timestamp`, `status`, `cpu_usage`, `memory_usage`, `uptime`
- `AgentHealthStatus` — enum: `Healthy`, `Degraded`, `Critical`, `Unknown`
- `HeartbeatReporter` — sends periodic health updates using `sysinfo` crate metrics

## HTTP Client & User-Agent

**CRITICAL**: Every outgoing HTTP/HTTPS request MUST carry the agent `User-Agent` header.

- Format: `smotra-agent/<CARGO_PKG_VERSION> (+https://github.com/smotra-monitoring/agent; <os>; <arch>)`, produced by `http_client::user_agent()` in `src/http_client/`.
- Build every `reqwest::Client` with `http_client::build_client(timeout, verify_tls)`. Never use `reqwest::Client::new()` or a bare `reqwest::Client::builder()` in `src/`.
- Exception: `octocrab` (self-upgrade) always sends `user-agent: octocrab`, and `add_header` would append a second `User-Agent` line, so do not use it. Leave the default until octocrab is replaced by a `build_client`-based GitHub client.
- Never put secrets, API keys, agent IDs or hostnames in the User-Agent.
- `examples/` and plugins cannot reach the private module; they must set their own `smotra-agent/<version> (<name>/<version>)` style UA.
- Tests that mock HTTP endpoints with `mockito` should `match_header("user-agent", ...)`.

## Configuration Versioning and Server Polling

The `Config` struct has a `version: u32` field (starts at 1) for synchronization with the server.

**TODO — Config Polling from Server** (not yet implemented):
- Agent periodically polls server for updated configuration.
- Include current version in `X-Config-Version` HTTP request header.
- Server returns `304 Not Modified` if versions match; otherwise returns new config with updated version.
- Agent validates, applies, and persists the new config to disk.
- On config update, reload monitoring tasks with new endpoints/settings.
- Skip polling if server unreachable. Use exponential backoff on failures.
