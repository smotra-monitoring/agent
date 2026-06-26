//! Agent auto-updater binary

use clap::Parser;
use octocrab::Octocrab;
use semver::Version;
use smotra::self_upgrade::{
    download_release_binary, fetch_latest_version, replace_binary_and_restart,
};
use smotra::Error;
use smotra::Result;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "smotra-updater")]
#[command(about = "Smotra Agent Auto-Updater", long_about = None)]
struct Cli {
    /// Update server URL
    #[arg(short, long)]
    server_url: String,

    /// Current version
    #[arg(short, long)]
    version: String,

    /// Check for updates only (don't install)
    #[arg(long)]
    check_only: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let current_version = Version::parse(cli.version.trim_start_matches('v'))
        .map_err(|e| Error::Config(format!("invalid --version '{}': {}", cli.version, e)))?;

    let octocrab = Octocrab::builder()
        .build()
        .map_err(|e| Error::GithubApi(format!("Failed to initialize GitHub client: {}", e)))?;

    info!("Checking for updates at {}", cli.server_url);
    let latest = fetch_latest_version(&octocrab, &cli.server_url).await?;
    let update_available = latest > current_version;

    if cli.check_only {
        if update_available {
            info!(
                "Update available: current={} latest={}",
                current_version, latest
            );
        } else {
            info!(
                "No update available: current={} latest={}",
                current_version, latest
            );
        }
        return Ok(());
    }

    if !update_available {
        info!("Already up to date: {}", current_version);
        return Ok(());
    }

    info!("Downloading update {}", latest);
    let extracted_binary = download_release_binary(&octocrab, &cli.server_url, &latest).await?;

    if let Err(e) = replace_binary_and_restart(&extracted_binary) {
        error!("Update failed: {}", e);
        return Err(e);
    }

    Ok(())
}
