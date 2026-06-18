# Self Upgrade

## Overview
The agent now supports self-upgrade through a Tokio background task. The task periodically checks a release endpoint, downloads a new binary when a higher version is available, verifies checksum integrity, replaces the running executable, and triggers a restart via the OS service manager.

The feature is intentionally disabled when running in containers. In containerized deployments, rolling updates should be performed by the orchestrator (for example, Kubernetes).

## Usage

### Basic Example
```rust
use parking_lot::RwLock;
use smotra::{Config, updater};
use std::sync::Arc;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() {
    let config = Arc::new(RwLock::new(Config::default()));
    let (_tx, rx) = broadcast::channel(1);

    let _ = updater::run_update_checker(config, rx).await;
}
```

### Configuration
```toml
[update]
enabled = true
check_url = "https://github.com/smotra-monitoring/agent"
check_interval_secs = 3600
```

## Integration
- The updater task is started by `Agent::start()` and shut down via the same broadcast shutdown channel as monitor/reporter tasks.
- `UpdateConfig` is generated from the OpenAPI spec and required in `AgentConfig`.
- On Linux, restart is triggered with `systemctl restart smotra` and then process exit.
- On Windows, restart uses a detached command shell launcher and process exit.

## Edge Cases and Pitfalls
- Containerized runtime (`/.dockerenv` or `CONTAINER` env var): updater is disabled.
- `check_interval_secs < 60` is clamped to 60 seconds to avoid excessive polling.
- If checksum validation fails, update is rejected.
- If release archive does not contain `smotra`/`agent` executable, update is rejected.
- If service-manager restart command fails, the current process still exits after replacement; ensure your service has restart policy configured.

## API Reference
Key APIs:
- `smotra::updater::run_update_checker`
- `smotra::updater::fetch_latest_version`
- `smotra::updater::download_release_binary`
- `smotra::updater::replace_binary_and_restart`

OpenAPI schema source:
- `api/openapi/api/spec.yaml` (`UpdateConfig` required in `AgentConfig`)

## Common Pitfalls
- Using a release host that does not expose `releases/latest/version.txt`.
- Publishing archives without matching `.sha256` file format (`<sha256> <filename>` expected).
- Running under systemd without an automatic restart policy.
