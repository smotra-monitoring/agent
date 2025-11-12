//! Example plugin demonstrating how to extend agent functionality

use anyhow::Result;
use async_trait::async_trait;
use smotra_agent::{
    plugin::MonitoringPlugin,
    {CheckType, Endpoint, MonitoringResult},
};
use std::collections::HashMap;

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
        "http_check"
    }

    fn version(&self) -> &str {
        "0.1.0"
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

                MonitoringResult {
                    id: uuid::Uuid::new_v4(),
                    agent_id: agent_id.to_string(),
                    target: endpoint.clone(),
                    check_type: CheckType::Plugin("http_check".to_string()),
                    success,
                    response_time_ms: Some(duration.as_secs_f64() * 1000.0),
                    error: if success {
                        None
                    } else {
                        Some(format!("HTTP {}", response.status()))
                    },
                    timestamp: chrono::Utc::now(),
                    metadata,
                }
            }
            Err(e) => MonitoringResult {
                id: uuid::Uuid::new_v4(),
                agent_id: agent_id.to_string(),
                target: endpoint.clone(),
                check_type: CheckType::Plugin("http_check".to_string()),
                success: false,
                response_time_ms: None,
                error: Some(e.to_string()),
                timestamp: chrono::Utc::now(),
                metadata: HashMap::new(),
            },
        };

        Ok(result)
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
