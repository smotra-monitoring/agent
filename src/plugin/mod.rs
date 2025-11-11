//! Plugin system for extending agent functionality

use crate::error::Result;
use crate::types::{Endpoint, MonitoringResult};
use async_trait::async_trait;

/// Trait for implementing monitoring plugins
#[async_trait]
pub trait MonitoringPlugin: Send + Sync {
    /// Name of the plugin
    fn name(&self) -> &str;

    /// Version of the plugin
    fn version(&self) -> &str;

    /// Perform a monitoring check
    async fn check(&self, agent_id: &str, endpoint: &Endpoint) -> Result<MonitoringResult>;

    /// Initialize the plugin
    async fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    /// Cleanup the plugin
    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Plugin registry for managing plugins
pub struct PluginRegistry {
    plugins: Vec<Box<dyn MonitoringPlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a new plugin
    pub fn register(&mut self, plugin: Box<dyn MonitoringPlugin>) {
        self.plugins.push(plugin);
    }

    /// Get a plugin by name
    pub fn get(&self, name: &str) -> Option<&dyn MonitoringPlugin> {
        self.plugins
            .iter()
            .find(|p| p.name() == name)
            .map(|p| p.as_ref())
    }

    /// List all registered plugins
    pub fn list(&self) -> Vec<(&str, &str)> {
        self.plugins
            .iter()
            .map(|p| (p.name(), p.version()))
            .collect()
    }

    /// Initialize all plugins
    pub async fn initialize_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.initialize().await?;
        }
        Ok(())
    }

    /// Shutdown all plugins
    pub async fn shutdown_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.shutdown().await?;
        }
        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
