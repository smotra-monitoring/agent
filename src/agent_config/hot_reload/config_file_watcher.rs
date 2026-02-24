//! Configuration file watching functionality
//!
//! Provides file system watching for config file changes.
//! Sends reload triggers through provided channel when changes are detected.

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, NoCache};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::error::{Error, Result};

use super::orchestrator::ReloadTrigger;

/// Configuration file watcher
///
/// Watches for config file changes and sends reload triggers through provided channel
pub(super) struct ConfigFileWatcher {
    config_path: PathBuf,
    // required for lifetime management of the file watcher - must be kept alive while watching
    file_watcher: Option<Debouncer<RecommendedWatcher, NoCache>>,
    trigger_tx: mpsc::UnboundedSender<ReloadTrigger>,
}

impl ConfigFileWatcher {
    /// Create a new config file watcher
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the configuration file to watch
    /// * `trigger_tx` - Channel to send reload triggers
    /// * `shutdown_rx` - Channel to receive shutdown signals
    pub(super) fn new(
        config_path: PathBuf,
        trigger_tx: mpsc::UnboundedSender<ReloadTrigger>,
    ) -> Result<Self> {
        Ok(Self {
            config_path,
            file_watcher: None,
            trigger_tx,
        })
    }

    /// Start watching for config changes
    ///
    /// Sets up file system watcher to monitor the config file.
    /// When changes are detected, sends ReloadTrigger::FileChange through trigger_tx.
    pub(super) fn start_watching(&mut self) -> Result<()> {
        // Set up file watcher
        let trigger_tx = self.trigger_tx.clone();
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
                            let _ = trigger_tx.send(ReloadTrigger::FileChange());
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_config_file_watcher_creation() {
        let temp_file = NamedTempFile::new().unwrap();

        let config = Config {
            agent_id: uuid::Uuid::new_v4(),
            ..Config::default()
        };
        config.save_to_file_secure(temp_file.path()).await.unwrap();

        let (trigger_tx, _trigger_rx) = mpsc::unbounded_channel();

        let watcher = ConfigFileWatcher::new(temp_file.path().to_path_buf(), trigger_tx);
        assert!(watcher.is_ok());
    }

    #[tokio::test]
    async fn test_start_watching() {
        let temp_file = NamedTempFile::new().unwrap();

        let config = Config {
            agent_id: uuid::Uuid::new_v4(),
            ..Config::default()
        };
        config.save_to_file_secure(temp_file.path()).await.unwrap();

        let (trigger_tx, _trigger_rx) = mpsc::unbounded_channel();

        let mut watcher =
            ConfigFileWatcher::new(temp_file.path().to_path_buf(), trigger_tx).unwrap();

        // Should be able to start watching without error
        let result = watcher.start_watching();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_watching_event() {
        let temp_file = NamedTempFile::new().unwrap();

        let config = Config {
            version: 1,
            agent_id: uuid::Uuid::new_v4(),
            agent_name: "Test Agent".to_string(),
            ..Config::default()
        };
        config.save_to_file_secure(temp_file.path()).await.unwrap();

        let (trigger_tx, mut trigger_rx) = mpsc::unbounded_channel();

        let mut watcher =
            ConfigFileWatcher::new(temp_file.path().to_path_buf(), trigger_tx).unwrap();

        // Should be able to start watching without error
        let result = watcher.start_watching();
        assert!(result.is_ok());

        let config_new = Config {
            version: 2,
            agent_id: uuid::Uuid::new_v4(),
            agent_name: "Test Agent 2".to_string(),
            ..Config::default()
        };
        config_new
            .save_to_file_secure(temp_file.path())
            .await
            .unwrap();

        if let ReloadTrigger::FileChange() = trigger_rx.recv().await.unwrap() {
        } else {
            panic!("Expected FileChange trigger");
        }
    }
}
