//! Configuration hot-reload functionality
//!
//! Provides mechanisms for reloading agent configuration in response to:
//! - File system changes to the config file
//! - SIGHUP signal on Unix systems
//! - Server-initiated config version changes (future implementation)

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, NoCache};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info};

use super::Config;
use crate::error::{Error, Result};

/// Events that trigger config reload
#[derive(Debug, Clone)]
pub enum ReloadTrigger {
    /// Config file was modified on filesystem
    FileChange(PathBuf),
    /// SIGHUP signal received (Unix only)
    Signal,
    /// Server reported a new config version (future implementation)
    ServerVersionChange(u32),
    /// Manual trigger (for testing or manual reloads)
    Manual,
}

/// Configuration reload manager
///
/// Watches for config file changes and signals, triggers reload callbacks
pub struct ConfigReloadManager {
    config_path: PathBuf,
    file_watcher: Option<Debouncer<RecommendedWatcher, NoCache>>,
    reload_tx: mpsc::UnboundedSender<ReloadTrigger>,
    reload_rx: mpsc::UnboundedReceiver<ReloadTrigger>,
    shutdown_rx: broadcast::Receiver<()>,
}

impl ConfigReloadManager {
    /// Create a new config reload manager
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the configuration file to watch
    /// * `shutdown_rx` - Channel to receive shutdown signals
    pub fn new(config_path: PathBuf, shutdown_rx: broadcast::Receiver<()>) -> Result<Self> {
        let (reload_tx, reload_rx) = mpsc::unbounded_channel();

        Ok(Self {
            config_path,
            file_watcher: None,
            reload_tx,
            reload_rx,
            shutdown_rx,
        })
    }

    /// Get a clone of the reload trigger sender
    ///
    /// This can be used by other components (like SIGHUP handler) to trigger reloads
    pub fn reload_sender(&self) -> mpsc::UnboundedSender<ReloadTrigger> {
        self.reload_tx.clone()
    }

    /// Start watching for config changes
    ///
    /// Returns immediately after setting up watchers. Use `run()` to process events.
    pub fn start_watching_file(&mut self) -> Result<()> {
        // Set up file watcher
        let reload_tx = self.reload_sender();
        let config_path = self.config_path.clone();

        let mut debouncer = new_debouncer(
            Duration::from_millis(500), // Debounce duration
            None,                       // No custom tick rate
            move |result: DebounceEventResult| match result {
                Ok(events) => {
                    for event in events {
                        debug!("File event: {:?}", event);
                        // Check if the event affects our config file
                        if event
                            .paths
                            .iter()
                            .any(|p| p.ends_with(config_path.file_name().unwrap_or_default()))
                        {
                            info!("Config file change detected: {:?}", config_path);
                            let _ = reload_tx.send(ReloadTrigger::FileChange(config_path.clone()));
                        }
                    }
                }
                Err(errors) => {
                    for error in errors {
                        error!("File watcher error: {:?}", error);
                    }
                }
            },
        )
        .map_err(|e| Error::Config(format!("Failed to create file watcher: {}", e)))?;

        // Watch the parent directory (some editors replace files instead of modifying them)
        let watch_path = self.config_path.parent().unwrap_or_else(|| Path::new("."));

        debouncer
            .watch(watch_path, RecursiveMode::NonRecursive)
            .map_err(|e| Error::Config(format!("Failed to watch config directory: {}", e)))?;

        info!("Started watching config directory: {:?}", watch_path);
        self.file_watcher = Some(debouncer);

        Ok(())
    }

    /// Run the reload manager event loop
    ///
    /// Processes reload triggers and invokes the callback function for each trigger.
    /// Runs until a shutdown signal is received.
    ///
    /// # Arguments
    ///
    /// * `callback` - Async function to call when reload is triggered.
    ///   Receives the reload trigger and should return Result<()>.
    pub async fn run<F, Fut>(mut self, callback: F) -> Result<()>
    where
        F: Fn(ReloadTrigger) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        info!("Config reload manager started");

        loop {
            tokio::select! {
                Some(trigger) = self.reload_rx.recv() => {
                    info!("Config reload triggered: {:?}", trigger);

                    match callback(trigger.clone()).await {
                        Ok(()) => {
                            info!("Config reload completed successfully");
                        }
                        Err(e) => {
                            error!("Config reload failed: {}", e);
                        }
                    }
                }
                _ = self.shutdown_rx.recv() => {
                    info!("Config reload manager shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Manually trigger a config reload
    ///
    /// Useful for testing or manual reload scenarios
    pub fn trigger_reload(&self, trigger: ReloadTrigger) -> Result<()> {
        self.reload_tx
            .send(trigger)
            .map_err(|e| Error::Config(format!("Failed to send reload trigger: {}", e)))?;
        Ok(())
    }
}

/// Handle SIGHUP signal and trigger config reload
///
/// Unix-only function that listens for SIGHUP and sends reload triggers.
///
/// # Arguments
///
/// * `reload_tx` - Channel to send reload triggers
/// * `shutdown_rx` - Channel to receive shutdown signals
#[cfg(unix)]
pub async fn handle_sighup(
    reload_tx: mpsc::UnboundedSender<ReloadTrigger>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sighup = signal(SignalKind::hangup())
        .map_err(|e| Error::Config(format!("Failed to setup SIGHUP handler: {}", e)))?;

    info!("SIGHUP handler started");

    loop {
        tokio::select! {
            _ = sighup.recv() => {
                info!("SIGHUP received, triggering config reload");
                reload_tx.send(ReloadTrigger::Signal)
                    .map_err(|e| Error::Config(format!("Failed to send reload trigger: {}", e)))?;
            }
            _ = shutdown_rx.recv() => {
                info!("SIGHUP handler shutting down");
                break;
            }
        }
    }

    Ok(())
}

/// Handle SIGHUP signal (no-op on non-Unix systems)
#[cfg(not(unix))]
pub async fn handle_sighup(
    _reload_tx: mpsc::UnboundedSender<ReloadTrigger>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    warn!("SIGHUP handler not supported on this platform");
    let _ = shutdown_rx.recv().await;
    Ok(())
}

/// infoer function to load and validate config from file
pub fn load_and_validate_config(path: &Path) -> Result<Config> {
    info!("Loading config from: {:?}", path);
    let config = Config::from_file(path)?;
    config.validate()?;
    info!("Config loaded and validated successfully");
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;
    use tempfile::TempDir;
    use tokio::sync::broadcast;
    use uuid::Uuid;

    fn create_test_config_file(dir: &Path, content: &str) -> PathBuf {
        let config_path = dir.join("test_config.toml");
        fs::write(&config_path, content).unwrap();
        config_path
    }

    #[test]
    fn test_reload_trigger_debug() {
        let trigger = ReloadTrigger::Signal;
        let _ = format!("{:?}", trigger); // Ensure Debug is implemented
    }

    #[tokio::test]
    async fn test_config_reload_manager_creation() {
        let temp_file = NamedTempFile::new().unwrap();

        let config = Config::default();
        config.save_to_file_secure(temp_file.path()).await.unwrap();

        let (_, shutdown_rx) = broadcast::channel(1);
        let manager = ConfigReloadManager::new(temp_file.path().to_path_buf(), shutdown_rx);
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_manual_trigger_reload() {
        let temp_file = NamedTempFile::new().unwrap();

        let config = Config::default();
        config.save_to_file_secure(temp_file.path()).await.unwrap();

        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let manager =
            ConfigReloadManager::new(temp_file.path().to_path_buf(), shutdown_rx).unwrap();

        // Clone the reload_tx for testing
        let reload_tx_clone = manager.reload_tx.clone();

        // Spawn the manager task
        let handle = tokio::spawn(async move {
            let callback = |trigger: ReloadTrigger| async move {
                info!("Test callback received trigger: {:?}", trigger);
                Ok(())
            };
            manager.run(callback).await
        });

        // Trigger a reload
        reload_tx_clone
            .send(ReloadTrigger::Manual)
            .expect("Failed to trigger reload");

        // Give it time to process
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Shutdown
        let _ = shutdown_tx.send(());

        // Wait for task to complete
        let _ = tokio::time::timeout(Duration::from_millis(100), handle).await;
    }

    #[tokio::test]
    async fn test_load_and_validate_config() {
        let temp_file = NamedTempFile::new().unwrap();

        let config = Config {
            version: 1,
            agent_name: "test".to_string(),
            agent_id: Uuid::new_v4(),
            ..Config::default()
        };

        config.save_to_file_secure(temp_file.path()).await.unwrap();

        let result = load_and_validate_config(temp_file.path());
        if let Err(ref e) = result {
            eprintln!("Config load error: {}", e);
        }
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.version, 1);
        assert_eq!(config.agent_name, "test");
    }

    #[tokio::test]
    async fn test_load_invalid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config_file(
            temp_dir.path(),
            r#"
# Invalid TOML - missing closing bracket
version = 1
agent_id = "123e4567-e89b-12d3-a456-426614174000"
agent_name = "test"
tags = [
        "#,
        );

        let result = load_and_validate_config(&config_path);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_config_nil_agent_id() {
        let temp_file = NamedTempFile::new().unwrap();

        let config = Config::default();

        config.save_to_file_secure(temp_file.path()).await.unwrap();
        let result = load_and_validate_config(temp_file.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("agent_id cannot be nil"));
    }

    #[tokio::test]
    async fn test_reload_trigger_variants() {
        // Test all variants can be created
        let _file_trigger = ReloadTrigger::FileChange(PathBuf::from("/tmp/config.toml"));
        let _signal_trigger = ReloadTrigger::Signal;
        let _server_trigger = ReloadTrigger::ServerVersionChange(2);
        let _manual_trigger = ReloadTrigger::Manual;
    }
}
