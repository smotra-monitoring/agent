//! Agent auto-updater binary

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "agent-updater")]
#[command(about = "Smotra Agent Auto-Updater", long_about = None)]
struct Cli {
    /// Update server URL
    #[arg(short, long)]
    server: String,

    /// Current version
    #[arg(short, long)]
    version: String,

    /// Installation directory
    #[arg(short, long, default_value = ".")]
    install_dir: PathBuf,

    /// Check for updates only (don't install)
    #[arg(long)]
    check_only: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    println!("Smotra Agent Auto-Updater");
    println!("=========================\n");
    println!("Current version: {}", cli.version);
    println!("Update server: {}", cli.server);

    // TODO: Implement update checking logic
    println!("\nChecking for updates...");

    if cli.check_only {
        todo!("No updates available (updater not yet implemented)");
    }

    // TODO: Implement update download and installation
    todo!("Update functionality coming soon!");
}
