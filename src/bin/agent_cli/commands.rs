//! Command handlers for CLI operations

use smotra_agent::{Agent, Config, Result};
use std::path::PathBuf;

/// Show current status
pub async fn show_status(config_path: PathBuf) -> Result<()> {
    let config = Config::from_file(&config_path)?;
    let agent = Agent::new(config);
    let status = agent.status();

    println!("Agent Status:");
    println!("  Running: {}", status.is_running);
    println!("  Checks Performed: {}", status.checks_performed);
    println!("  Checks Successful: {}", status.checks_successful);
    println!("  Checks Failed: {}", status.checks_failed);
    println!("  Server Connected: {}", status.server_connected);
    println!("  Cached Results: {}", status.cached_results);

    Ok(())
}

/// Validate configuration
pub async fn validate_config(config_path: PathBuf) -> Result<()> {
    match Config::from_file(&config_path) {
        Ok(config) => match config.validate() {
            Ok(_) => println!("✓ Configuration is valid"),
            Err(e) => {
                eprintln!("✗ Configuration validation failed: {}", e);
                return Err(e);
            }
        },
        Err(e) => {
            eprintln!("✗ Failed to load configuration: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Generate default configuration
pub async fn generate_config(output: PathBuf) -> Result<()> {
    let config = Config::default();
    config.save_to_file(&output)?;
    println!("Generated default configuration at: {}", output.display());
    Ok(())
}
