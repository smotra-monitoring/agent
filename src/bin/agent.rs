//! Main agent daemon binary

use clap::Parser;
use smotra_agent::{Agent, Config, Endpoint, Result};
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "agent")]
#[command(about = "Smotra Agent - Distributed monitoring daemon", long_about = None)]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Generate default configuration and exit
    #[arg(long)]
    gen_config: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&cli.log_level)),
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    info!("Starting Smotra Agent");

    // Generate config if requested
    if cli.gen_config {
        let mut config = Config::default();

        config.endpoints.push(
            Endpoint::new("8.8.8.8").with_tags(vec!["DNS".to_string(), "google".to_string()]),
        );

        config.save_to_file(&cli.config)?;
        info!(
            "Generated default configuration at: {}",
            cli.config.display()
        );
        return Ok(());
    }

    // Load configuration
    let config = if cli.config.exists() {
        info!("Loading configuration from: {}", cli.config.display());
        match Config::from_file(&cli.config) {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to load configuration: {}", e);
                return Err(e.into());
            }
        }
    } else {
        error!("Configuration file not found: {}", cli.config.display());
        error!("Run with --gen-config to generate a default configuration");
        std::process::exit(1);
    };

    // Validate configuration
    if let Err(e) = config.validate() {
        error!("Configuration validation failed: {}", e);
        return Err(e.into());
    }

    info!("Configuration loaded successfully");
    info!("Agent ID: {}", config.agent_id);
    info!("Tags: {:?}", config.tags);
    info!("Monitoring {} endpoints", config.endpoints.len());

    // Create and start agent
    let agent = Agent::new(config);

    match agent.start().await {
        Ok(_) => {
            info!("Agent stopped gracefully");
            Ok(())
        }
        Err(e) => {
            error!("Agent error: {}", e);
            Err(e.into())
        }
    }
}
