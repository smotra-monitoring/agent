//! Main agent daemon binary

use clap::Parser;
use smotra::{
    handle_sighup, Agent, Claim, Config, ConfigReloadManager, Endpoint, Result,
};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};

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

        config.save_to_file_secure(&cli.config).await?;
        info!(
            "Generated default configuration at: {}",
            cli.config.display()
        );
        return Ok(());
    }

    // Load configuration
    let mut config = if cli.config.exists() {
        info!("Loading configuration from: {}", cli.config.display());
        match Config::from_file(&cli.config) {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to load configuration: {}", e);
                return Err(e);
            }
        }
    } else {
        error!("Configuration file not found: {}", cli.config.display());
        error!("Run with --gen-config to generate a default configuration");
        std::process::exit(1);
    };

    // Check if API key is configured
    if !config.server.is_configured() {
        if config.server.url.is_empty() {
            error!("Server URL not configured. Please set 'server.url' in the configuration file.");
            std::process::exit(1);
        };

        info!("Starting agent claiming workflow, due to missing API key ...");

        // Run claiming workflow
        let claim = Claim::new(&config);
        match claim.run().await {
            Ok(claim_result) => {
                info!("Claiming workflow completed successfully");
                info!("Agent ID: {}", claim_result.agent_id);

                // Apply claim result to config
                config.apply_claim_result(claim_result);

                // Save updated config securely
                config.save_to_file_secure(&cli.config).await?;
                info!("Configuration saved to: {}", cli.config.display());
            }
            Err(e) => {
                error!("Claiming workflow failed: {}", e);
                return Err(e);
            }
        }
    } else {
        info!("API key found in configuration");
    }

    // Validate configuration
    if let Err(e) = config.validate() {
        error!("Configuration validation failed: {}", e);
        return Err(e);
    }

    info!("Configuration loaded successfully");
    info!("Agent ID: {}", config.agent_id);
    info!("Tags: {:?}", config.tags);
    info!("Monitoring {} endpoints", config.endpoints.len());

    // -----------------------------
    // remove block
    // -----------------------------

    let agent = Arc::new(Agent::new(config));

    // Set up config reload manager
    let config_path_clone = cli.config.clone();
    let agent_clone = Arc::clone(&agent);

    let reload_handle = {
        let shutdown_rx = agent_clone.subscribe_shutdown();

        // Create config reload manager
        let mut reload_manager = ConfigReloadManager::new(config_path_clone.clone(), shutdown_rx)
            .unwrap_or_else(|e| {
                error!("Failed to create config reload manager: {}", e);
                std::process::exit(1);
            });

        // Start watching for file changes
        if let Err(e) = reload_manager.start_watching_file() {
            warn!("Failed to start config file watching: {}", e);
            warn!("Config hot-reload from file changes will not be available");
        } else {
            info!("Config file watching enabled");
        }

        // Clone for the reload callback closure
        let agent_for_reload = Arc::clone(&agent_clone);
        let config_path_for_reload = config_path_clone.clone();

        // Spawn reload manager task
        tokio::spawn(async move {
            reload_manager
                .run(move |trigger| {
                    let agent = Arc::clone(&agent_for_reload);
                    let config_path = config_path_for_reload.clone();
                    async move {
                        info!("Config reload triggered: {:?}", trigger);

                        // Load and validate new config
                        match Config::load_and_validate_config(&config_path) {
                            Ok(new_config) => {
                                // Apply the new config
                                match agent.reload_config(new_config) {
                                    Ok(()) => {
                                        info!("Config reload completed successfully");
                                        Ok(())
                                    }
                                    Err(e) => {
                                        error!("Failed to apply reloaded config: {}", e);
                                        Err(e)
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to load config during reload: {}", e);
                                Err(e)
                            }
                        }
                    }
                })
                .await
        })
    };

    // Set up SIGHUP handler
    let sighup_handle = {
        let shutdown_rx = agent_clone.subscribe_shutdown();
        let reload_manager =
            ConfigReloadManager::new(cli.config.clone(), shutdown_rx.resubscribe()).unwrap();
        let reload_tx = reload_manager.reload_sender();

        tokio::spawn(async move {
            if let Err(e) = handle_sighup(reload_tx, shutdown_rx).await {
                error!("SIGHUP handler error: {}", e);
            }
        })
    };

    info!("Config hot-reload enabled (file changes and SIGHUP)");

    // -----------------------------
    // remove block
    // -----------------------------

    // Create and start agent
    // let agent = Agent::new(config);

    match agent.start().await {
        Ok(_) => {
            info!("Agent stopped gracefully");

            // TODO: remove
            // Wait for reload tasks to complete (with short timeout)
            let timeout_duration = std::time::Duration::from_secs(2);
            let _ = tokio::time::timeout(timeout_duration, async {
                let _ = tokio::join!(reload_handle, sighup_handle);
            })
            .await;

            Ok(())
        }
        Err(e) => {
            error!("Agent error: {}", e);
            Err(e)
        }
    }
}
