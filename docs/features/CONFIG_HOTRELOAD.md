# Configuration Hot-Reload

This document describes the configuration hot-reload functionality in the Smotra Agent, which allows configuration changes to be applied without restarting the agent process.

## Overview

The agent supports automatic configuration reloading in response to:
1. **File system changes** - When the config file is modified
2. **SIGHUP signal** - On Unix systems, sending SIGHUP triggers a reload (not available on Windows)
3. **Server version changes** - Future implementation for server-initiated updates

## Architecture

### Components

#### ConfigReloadManager
The `ConfigReloadManager` is the core component that:
- Watches the config file directory for changes using the `notify` crate
- Debounces file system events to avoid multiple reloads
- Provides a unified interface for triggering reloads from different sources
- Runs an event loop that processes reload triggers and invokes callbacks

#### ReloadTrigger
An enum representing the source of a reload request:
```rust
pub enum ReloadTrigger {
    FileChange(PathBuf),        // Config file was modified
    Signal,                     // SIGHUP received (Unix only)
    ServerVersionChange(u32),   // Server reported new version (future)
}
```

#### Agent::reload_config()
The `Agent` struct provides a `reload_config()` method that:
- Validates the new configuration before applying it
- Logs significant configuration changes
- Updates the configuration atomically using `RwLock`
- Preserves the old config if validation fails

### Workflow

```
┌─────────────────┐
│  Config File    │
│  Modified       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐       ┌─────────────────┐
│  File Watcher   │──────▶│ ConfigReload    │
│  (notify crate) │       │ Manager         │
└─────────────────┘       └────────┬────────┘
                                   │
┌─────────────────┐                │
│  SIGHUP Signal  │────────────────┤
│  (Unix only)    │                │
└─────────────────┘                │
                                   ▼
                          ┌─────────────────┐
                          │ Reload Callback │
                          │ - Load config   │
                          │ - Validate      │
                          │ - Apply         │
                          └────────┬────────┘
                                   │
                                   ▼
                          ┌─────────────────┐
                          │ Agent           │
                          │ reload_config() │
                          └─────────────────┘
```

## Usage

### Basic Setup

The main binary (`smotra`) automatically sets up config hot-reload when starting:

```rust
// Create agent
let agent = Arc::new(Agent::new(config));

// Set up reload manager with file watching
let mut reload_manager = ConfigReloadManager::new(
    config_path.clone(), 
    agent.subscribe_shutdown()
)?;

reload_manager.start_watching()?;

// Run reload manager with callback
reload_manager.run(|trigger| {
    let agent = agent.clone();
    let config_path = config_path.clone();
    async move {
        let new_config = load_and_validate_config(&config_path)?;
        agent.reload_config(new_config)?;
        Ok(())
    }
}).await;
```

### Manual Reload Trigger

You can also manually trigger a reload:

```rust
let reload_manager = ConfigReloadManager::new(config_path, shutdown_rx)?;
reload_manager.trigger_reload(ReloadTrigger::Signal)?;
```

### SIGHUP Handler (Unix)

On Unix systems, the agent automatically listens for SIGHUP:

```bash
# Send SIGHUP to reload config
kill -HUP <agent_pid>

# Or using pkill
pkill -HUP smotra
```

The SIGHUP handler is set up automatically in the main binary:

```rust
let sighup_handle = tokio::spawn(async move {
    handle_sighup(reload_tx, shutdown_rx).await
});
```

## Configuration Changes

### What Gets Reloaded

When a config reload is triggered, the following changes take effect:

✅ **Immediate effect:**
- Monitoring interval
- Monitoring timeout
- Ping count
- Endpoint list (picked up on next monitoring cycle)
- Tags
- Agent name
- Storage settings

⚠️ **Requires attention:**
- Server URL changes (may affect ongoing requests)
- Agent ID changes (logged as warning, may cause server issues)
- API key changes (may affect authentication)

### Validation

Before applying a new configuration, the following validations are performed:

1. **TOML parsing** - File must be valid TOML
2. **Schema validation** - All required fields must be present
3. **Business rules:**
   - `agent_id` must not be nil UUID
   - `monitoring.interval_secs` must be greater than 0
   - Server URL must be valid (if configured)

If validation fails, the old configuration remains in effect and an error is logged.

### Atomic Updates

Configuration updates are atomic - either the entire new config is applied, or the old config remains unchanged. This prevents partial updates that could leave the agent in an inconsistent state.

## Monitoring and Logging

### Log Messages

The reload system provides detailed logging:

```
INFO  Config file change detected: /etc/smotra/config.toml
INFO  Attempting to reload configuration (version: 2)
INFO  Monitoring interval changed: 60 -> 120 seconds
INFO  Number of endpoints changed: 3 -> 5
INFO  Configuration reloaded successfully (version: 2)
```

### Error Handling

Errors during reload are logged but don't crash the agent:

```
ERROR Failed to load config during reload: Failed to parse config: ...
ERROR Config reload failed: agent_id cannot be nil UUID
```

The agent continues running with the previous valid configuration.

## Examples

### Example 1: Changing Monitoring Interval

1. Edit `config.toml`:
```toml
[monitoring]
interval_secs = 120  # Changed from 60
```

2. Save the file - reload happens automatically

3. Check logs:
```
INFO  Config file change detected: config.toml
INFO  Monitoring interval changed: 60 -> 120 seconds
INFO  Configuration reloaded successfully (version: 1)
```

### Example 2: Adding New Endpoints

1. Edit `config.toml`:
```toml
[[endpoints]]
target = "192.168.1.1"
name = "Router"
tags = ["network", "critical"]
check_type = "Ping"

[[endpoints]]
target = "8.8.8.8"
name = "Google DNS"
tags = ["external", "dns"]
check_type = "Ping"
```

2. Send SIGHUP:
```bash
pkill -HUP smotra
```

3. Monitoring coordinator picks up new endpoints in next cycle

### Example 3: Server Configuration Update

1. Edit `config.toml`:
```toml
[server]
url = "https://new-server.example.com"
api_key = "new-api-key"
```

2. File watcher triggers reload automatically

3. Next heartbeat and report use new server URL

## Implementation Details

### File Watching

- Uses `notify` crate's recommended watcher (platform-specific)
- Watches the parent directory (not the file directly) to handle editor behavior
- Debounces events with 500ms window to avoid rapid-fire reloads
- Recursive mode is disabled (only watches the config directory)

### Debouncing

File system events are debounced to handle:
- Editors that save files by write-replace (creates temporary files)
- Multiple events from a single save operation
- Network file systems with delayed write notifications

The default debounce duration is 500ms, which works well for most scenarios.

### Thread Safety

Configuration updates use `parking_lot::RwLock` for:
- Fast read access (monitoring tasks read config frequently)
- Exclusive write access during updates
- Lock-free reads in the common case

### Shutdown Coordination

The reload manager participates in graceful shutdown:
- Subscribes to agent's shutdown channel
- Stops watching files when shutdown is signaled
- Completes in-flight reload operations before exiting

## Future Enhancements

### Server-Initiated Config Updates

**Status**: Planned - see `ReloadTrigger::ServerVersionChange(u32)`

The agent will periodically poll the server for config updates:

1. Agent includes current config version in heartbeat or dedicated polling
2. Server compares version numbers
3. If server version is newer, it returns updated config
4. Agent reloads with server-provided config

Implementation considerations:
- Exponential backoff for polling failures
- Conflict resolution (file changes vs server changes)
- Security (verify server's authority to update config)
- Audit logging for server-initiated changes

### Dynamic Monitoring Task Restart

Currently, monitoring tasks pick up config changes on their next cycle. A future enhancement could:
- Restart monitoring coordinator immediately on endpoint changes
- Cancel in-flight checks that no longer apply
- Provide faster response to urgent config changes

### Config Rollback

Add ability to rollback to previous config if new config causes issues:
- Keep history of N previous configs
- Automatic rollback on repeated failures
- Manual rollback via CLI or signal

## Testing

### Unit Tests

The `reload.rs` module includes comprehensive unit tests:
- Manager creation and setup
- Manual trigger functionality
- Config loading and validation
- Error handling for invalid configs

### Integration Tests

The `config_reload_integration_tests.rs` file provides end-to-end tests:
- File change detection and reload
- Manual trigger reload
- Invalid config handling (keeps old config)
- Malformed TOML handling
- Multiple sequential reloads
- Different trigger types

Run tests with:
```bash
cargo test config_reload
cargo test --test config_reload_integration_tests
```

## Troubleshooting

### Config Not Reloading on File Change

**Problem**: Modified config file but no reload occurred

**Solutions**:
1. Check logs for file watcher errors
2. Verify config file is in the watched directory
3. Some editors (vim, emacs) may need special handling - they create backup files
4. Try manual trigger with SIGHUP: `pkill -HUP smotra`

### Reload Fails with Validation Error

**Problem**: Config reload attempted but failed validation

**Solutions**:
1. Check logs for specific validation error
2. Verify TOML syntax: `cat config.toml | toml-test`
3. Ensure all required fields are present
4. Check `agent_id` is not nil UUID
5. Verify `monitoring.interval_secs > 0`

### Old Config Still in Effect

**Problem**: New config loaded but old values still being used

**Solutions**:
1. Check logs to confirm reload succeeded
2. Some changes require monitoring cycle to complete (wait `interval_secs`)
3. For server URL changes, in-flight requests may still use old URL
4. Restart agent for guaranteed fresh start

### Permission Denied on Config File

**Problem**: File watcher cannot read config file or directory

**Solutions**:
1. Check file permissions: `ls -la config.toml`
2. Ensure agent process user has read access to config file
3. Ensure agent process user has read access to parent directory
4. On production systems, config should be owned by agent user with 0600 permissions

## Security Considerations

### File Permissions

Config files should have restrictive permissions:
```bash
chmod 600 config.toml
chown smotra:smotra config.toml
```

This prevents unauthorized users from:
- Reading sensitive information (API keys)
- Modifying configuration maliciously

### Validation is Critical

Always validate config before applying:
- Prevents agent from entering invalid state
- Protects against malformed or malicious configs
- Ensures business rules are enforced

### Audit Logging

All config reloads are logged for security auditing:
- Timestamp of reload
- Source of trigger (file, signal, server)
- Changes applied
- Success or failure

This provides an audit trail for configuration changes.

## Platform Support

| Platform | File Watching | SIGHUP | Notes |
|----------|---------------|--------|--------|
| Linux | ✅ Yes | ✅ Yes | Recommended platform |
| macOS | ✅ Yes | ✅ Yes | Fully supported |
| Windows | ✅ Yes | ❌ No | No SIGHUP, use file watching only |
| BSD | ✅ Yes | ✅ Yes | Should work but not extensively tested |

## Performance Impact

Config reload is designed to be lightweight:

- **File watching**: Minimal CPU usage, event-driven
- **Config reload**: Typically < 10ms for small configs
- **Memory**: Single extra config copy during reload
- **Monitoring tasks**: No interruption, gradual adoption of new config

Reloading a typical config (< 100 endpoints) has negligible performance impact.

## Related Documentation

- [Configuration Reference](PROJECT_STRUCTURE.md) - Complete config file format
- [Agent Architecture](README.md) - Overall agent design
- [OpenAPI Specification](../api/openapi/api/spec.yaml) - Server API for future server-initiated reloads
