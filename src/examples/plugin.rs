//! Example plugin demonstrating how to extend agent functionality

use anyhow::Result;
use async_trait::async_trait;
use smotra_agent::{
    plugin::MonitoringPlugin,
    {CheckType, Endpoint, MonitoringResult},
};
use std::collections::HashMap;

const PLUGIN_NAME: &str = "http_plugin";
const PLUGIN_VERSION: &str = "0.0.1";

/// Example HTTP monitoring plugin
struct HttpPlugin {
    client: reqwest::Client,
}

impl HttpPlugin {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl MonitoringPlugin for HttpPlugin {
    fn name(&self) -> &str {
        PLUGIN_NAME
    }

    fn version(&self) -> &str {
        PLUGIN_VERSION
    }

    async fn check(
        &self,
        agent_id: &str,
        endpoint: &Endpoint,
    ) -> smotra_agent::Result<MonitoringResult> {
        let url = if let Some(port) = endpoint.port {
            format!("http://{}:{}", endpoint.address, port)
        } else {
            format!("http://{}", endpoint.address)
        };

        let start = std::time::Instant::now();

        let result = match self.client.get(&url).send().await {
            Ok(response) => {
                let duration = start.elapsed();
                let success = response.status().is_success();

                let mut metadata = HashMap::new();
                metadata.insert(
                    "status_code".to_string(),
                    response.status().as_u16().to_string(),
                );
                metadata.insert("url".to_string(), url.clone());

                let result = MonitoringResult {
                    id: uuid::Uuid::new_v4(),
                    agent_id: agent_id.to_string(),
                    target: endpoint.clone(),
                    check_type: CheckType::Plugin(PLUGIN_NAME.into()),
                    success,
                    response_time_ms: Some(duration.as_millis() as f64),
                    error: if success {
                        None
                    } else {
                        Some(format!("HTTP {}", response.status()))
                    },
                    timestamp: chrono::Utc::now(),
                    metadata,
                };
                Ok(result)
            }
            Err(e) => Err(smotra_agent::Error::Http(e)),
        };

        result
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Smotra Agent Plugin Example");
    println!("===========================\n");

    let plugin = HttpPlugin::new();

    println!("Plugin: {} v{}", plugin.name(), plugin.version());

    // Example endpoint
    let endpoint = Endpoint::new("example.com").with_port(80);

    println!(
        "\nTesting endpoint: {}:{}",
        endpoint.address,
        endpoint.port.unwrap_or(80)
    );

    let result = plugin.check("example-agent", &endpoint).await?;

    println!("\nResult:");
    println!("  Success: {}", result.success);
    if let Some(time) = result.response_time_ms {
        println!("  Response Time: {:.2}ms", time);
    }
    if let Some(error) = result.error {
        println!("  Error: {}", error);
    }
    println!("  Metadata: {:?}", result.metadata);

    Ok(())
}
