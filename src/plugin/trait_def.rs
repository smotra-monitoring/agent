//! Plugin trait definition

use crate::core::{Endpoint, MonitoringResult};
use crate::error::Result;
use async_trait::async_trait;

/// Trait for implementing monitoring plugins
#[async_trait]
pub trait MonitoringPlugin: Send + Sync {
    /// Name of the plugin
    fn name(&self) -> &str;

    /// Version of the plugin
    fn version(&self) -> &str;

    /// Perform a monitoring check
    async fn check(&self, agent_id: &uuid::Uuid, endpoint: &Endpoint) -> Result<MonitoringResult>;

    /// Initialize the plugin
    async fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    /// Cleanup the plugin
    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}
