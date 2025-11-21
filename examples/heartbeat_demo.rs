//! Example demonstrating heartbeat metrics collection

use smotra_agent::{Config, HeartbeatReporter, MonitoringConfig, ServerConfig, StorageConfig};

fn main() {
    println!("=== Heartbeat Metrics Demo ===\n");

    // Create a test configuration
    let config = Config {
        agent_id: "demo-agent-001".to_string(),
        tags: vec!["demo".to_string()],
        monitoring: MonitoringConfig::default(),
        server: ServerConfig {
            url: Some("https://example.com".to_string()),
            api_key: Some("demo-key".to_string()),
            ..Default::default()
        },
        storage: StorageConfig::default(),
        endpoints: vec![],
    };

    // Create heartbeat reporter
    let reporter = match HeartbeatReporter::new(config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to create reporter: {}", e);
            return;
        }
    };

    // Collect metrics
    println!("Collecting system metrics...\n");
    let heartbeat = reporter.collect_metrics();

    // Display the metrics
    println!("Agent Heartbeat:");
    println!("  Timestamp: {}", heartbeat.timestamp);
    println!("  Status: {:?}", heartbeat.status);

    if let Some(cpu) = heartbeat.cpu_usage_percent {
        println!("  CPU Usage: {:.2}%", cpu);
    } else {
        println!("  CPU Usage: Not available");
    }

    if let Some(mem) = heartbeat.memory_usage_percent {
        println!("  Memory Usage: {:.2}%", mem);
    } else {
        println!("  Memory Usage: Not available");
    }

    // Show JSON representation
    println!("\nJSON representation:");
    match serde_json::to_string_pretty(&heartbeat) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Failed to serialize: {}", e),
    }
}
