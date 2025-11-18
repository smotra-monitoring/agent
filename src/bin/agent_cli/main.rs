//! Agent CLI with interactive TUI for monitoring and configuration

mod cli_args;
mod commands;
mod logging;
mod tui;

use clap::Parser;
use cli_args::{Cli, Commands};
use smotra_agent::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Tui) | None => {
            // For TUI mode, use in-memory log buffer
            let log_entries = logging::init_tui_logging(&cli.log_level);
            tui::start_tui(cli.config, log_entries).await?
        }
        Some(Commands::Status) => {
            // For non-TUI commands, use regular stdout logging
            logging::init_stdout_logging(&cli.log_level);
            commands::show_status(cli.config).await?
        }
        Some(Commands::ValidateConfig) => {
            logging::init_stdout_logging(&cli.log_level);
            commands::validate_config(cli.config).await?
        }
        Some(Commands::GenConfig { output }) => {
            logging::init_stdout_logging(&cli.log_level);
            commands::generate_config(output).await?
        }
    }

    Ok(())
}
