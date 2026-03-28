//! Main agent daemon binary

use clap::Parser;
use smotra::{Agent, Claim, Config, Endpoint, Result};
use std::path::{Path, PathBuf};
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "smotra")]
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

/// Initializes the tracing subscriber using the provided log level.
/// `RUST_LOG` environment variable takes precedence over `log_level`.
fn init_tracing(log_level: &str) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();
}

/// Generates a default configuration file at `path` with a sample endpoint and exits.
async fn generate_config(path: &Path) -> Result<()> {
    let mut config = Config::default();
    config
        .endpoints
        .push(Endpoint::new("8.8.8.8").with_tags(vec!["DNS".to_string(), "google".to_string()]));
    config.save_to_file_secure(path).await?;
    info!("Generated default configuration at: {}", path.display());
    Ok(())
}

/// Loads the configuration from `path`.
/// Exits the process with an error message if the file does not exist.
fn load_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        error!("Configuration file not found: {}", path.display());
        error!("Run with --gen-config to generate a default configuration");
        std::process::exit(1);
    }

    info!("Loading configuration from: {}", path.display());
    Config::from_file(path).map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })
}

/// Ensures the agent is claimed.
///
/// If the server API key is already present in `config`, this is a no-op.
/// Otherwise, it runs the interactive claiming workflow, applies the result
/// to `config`, and persists the updated config to `config_path`.
async fn ensure_claimed(config: &mut Config, config_path: &Path) -> Result<()> {
    if config.server.url.is_empty() {
        error!("Server URL not configured. Please set 'server.url' in the configuration file.");
        std::process::exit(1);
    }

    info!("Starting agent claiming workflow, due to missing API key ...");

    let claim = Claim::new(config);
    let claim_result = claim.run().await.map_err(|e| {
        error!("Claiming workflow failed: {}", e);
        e
    })?;

    info!("Claiming workflow completed successfully");
    info!("Agent ID: {}", claim_result.agent_id);

    config.apply_claim_result(claim_result);
    config.save_to_file_secure(config_path).await?;
    info!("Configuration saved to: {}", config_path.display());

    Ok(())
}

/// Validates config, logs a summary, then creates and runs the agent.
fn print_info(config: &Config) -> Result<()> {
    config.validate().map_err(|e| {
        error!("Configuration validation failed: {}", e);
        e
    })?;

    info!("Configuration loaded successfully");
    info!("Agent ID: {}", config.agent_id);
    info!("Tags: {:?}", config.tags);
    info!("Monitoring {} endpoints", config.endpoints.len());

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    init_tracing(&cli.log_level);

    if cli.gen_config {
        return generate_config(&cli.config).await;
    }

    let mut config = load_config(&cli.config)?;
    if config.server.is_claim_required() {
        ensure_claimed(&mut config, &cli.config).await?;
    }
    print_info(&config)?;

    info!("Starting the agent");

    let agent = Agent::new(cli.config)?;
    agent.start().await.map_err(|e| {
        error!("Agent error: {}", e);
        e
    })?;

    info!("Agent stopped gracefully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    mod generate_config_tests {
        use super::*;

        #[tokio::test]
        async fn creates_file_at_given_path() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("config.toml");

            let result = generate_config(&path).await;

            assert!(result.is_ok(), "generate_config should succeed");
            assert!(path.exists(), "config file should be created");
        }

        #[tokio::test]
        async fn written_file_is_valid_config() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("config.toml");

            generate_config(&path).await.unwrap();

            let config = Config::from_file(&path);
            assert!(
                config.is_ok(),
                "generated file should parse as valid Config"
            );
        }

        #[tokio::test]
        async fn generated_config_has_sample_endpoint() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("config.toml");

            generate_config(&path).await.unwrap();

            let config = Config::from_file(&path).unwrap();
            assert!(
                !config.endpoints.is_empty(),
                "generated config should contain at least one sample endpoint"
            );
        }
    }

    mod load_config_tests {
        use super::*;

        #[tokio::test]
        async fn loads_valid_config_file() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("config.toml");
            generate_config(&path).await.unwrap();

            let result = load_config(&path);

            assert!(
                result.is_ok(),
                "load_config should succeed for a valid file"
            );
        }

        #[test]
        fn returns_error_for_malformed_toml() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("bad.toml");
            std::fs::write(&path, b"not valid toml ][[[").unwrap();

            let result = load_config(&path);

            assert!(
                result.is_err(),
                "load_config should fail for malformed TOML"
            );
        }
    }
}
