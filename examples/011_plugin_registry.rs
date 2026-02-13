//! Simple example demonstrating plugin registry usage

use anyhow::Result;
use async_trait::async_trait;
use smotra::{
    MonitoringPlugin, PluginRegistry, {CheckType, Endpoint, MonitoringResult, PluginResult},
};
use std::collections::HashMap;

// Simple example plugin
struct DummyPlugin {
    name: String,
}

impl DummyPlugin {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait]
impl MonitoringPlugin for DummyPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn check(
        &self,
        agent_id: &uuid::Uuid,
        endpoint: &Endpoint,
    ) -> smotra::Result<MonitoringResult> {
        let plugin_result = PluginResult {
            plugin_name: self.name.clone(),
            plugin_version: "1.0.0".to_string(),
            success: true,
            response_time_ms: Some(42.0),
            error: None,
            data: HashMap::new(),
        };

        Ok(MonitoringResult {
            id: uuid::Uuid::new_v4(),
            agent_id: *agent_id,
            target: endpoint.clone(),
            check_type: CheckType::Plugin(plugin_result),
            timestamp: chrono::Utc::now(),
        })
    }

    async fn initialize(&mut self) -> smotra::Result<()> {
        println!("  {} initialized", self.name);
        Ok(())
    }

    async fn shutdown(&mut self) -> smotra::Result<()> {
        println!("  {} shutdown", self.name);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Plugin Registry Example");
    println!("=======================\n");

    // Create a new registry
    let mut registry = PluginRegistry::new();
    println!("Created empty plugin registry");

    // Register some plugins
    println!("\nRegistering plugins...");
    registry.register(Box::new(DummyPlugin::new("http_checker")));
    registry.register(Box::new(DummyPlugin::new("ssl_checker")));
    registry.register(Box::new(DummyPlugin::new("dns_checker")));

    // List all plugins
    println!("\nRegistered plugins:");
    for (name, version) in registry.list() {
        println!("  - {} v{}", name, version);
    }

    // Get a specific plugin
    println!("\nLooking up 'http_checker'...");
    if let Some(plugin) = registry.get("http_checker") {
        println!("  Found: {} v{}", plugin.name(), plugin.version());
    }

    // Initialize all plugins
    println!("\nInitializing all plugins...");
    registry.initialize_all().await?;

    // Use a plugin
    println!("\nUsing 'ssl_checker' plugin...");
    if let Some(plugin) = registry.get("ssl_checker") {
        let agent_id = uuid::Uuid::new_v4();
        let endpoint = Endpoint::new("example.com").with_port(443);

        let result = plugin.check(&agent_id, &endpoint).await?;
        println!(
            "  Check result: {}",
            if result.is_successful() {
                "✓ Success"
            } else {
                "✗ Failed"
            }
        );
    }

    // Shutdown all plugins
    println!("\nShutting down all plugins...");
    registry.shutdown_all().await?;

    println!("\nDone!");

    Ok(())
}
