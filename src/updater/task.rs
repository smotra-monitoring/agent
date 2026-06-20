use super::{download_release_binary, fetch_latest_version, is_newer_than_current};
use crate::agent_config::Config;
use crate::error::Result;
use crate::updater::environment::is_containerized;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{error, info, warn};

pub async fn run_update_checker(
    config: Arc<RwLock<Config>>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    if is_containerized() {
        info!("Running in containerized environment. Self-upgrade disabled");
        return Ok(());
    }

    let client = reqwest::Client::builder().build()?;

    let mut current_interval_secs = normalize_interval(config.read().update.check_interval_secs);
    let mut iv = interval(std::time::Duration::from_secs(current_interval_secs));
    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = iv.tick() => {
                let cfg = config.read().clone();

                let next_interval_secs = normalize_interval(cfg.update.check_interval_secs);
                if next_interval_secs != current_interval_secs {
                    current_interval_secs = next_interval_secs;
                    iv = interval(std::time::Duration::from_secs(current_interval_secs));
                    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    info!("Update check interval changed to {} seconds", current_interval_secs);
                }

                if !cfg.update.enabled {
                    continue;
                }

                match fetch_latest_version(&client, &cfg.update.check_url).await {
                    Ok(latest) => {
                        match is_newer_than_current(&latest) {
                            Ok(true) => {
                                info!("New version {} detected. Starting upgrade", latest);
                                match download_release_binary(&client, &cfg.update.check_url, &latest).await {
                                    Ok(new_binary) => {
                                        if let Err(e) = super::replace_binary_and_restart(&new_binary) {
                                            error!("Failed to replace/restart after update: {}", e);
                                        }
                                    }
                                    Err(e) => error!("Failed to download release {}: {}", latest, e),
                                }
                            }
                            Ok(false) => {}
                            Err(e) => error!("Failed to compare versions: {}", e),
                        }
                    }
                    Err(e) => {
                        warn!("Update check failed: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Update checker shutting down");
                break;
            }
        }
    }

    Ok(())
}

fn normalize_interval(raw_secs: i64) -> u64 {
    if raw_secs < 60 {
        return 60;
    }

    raw_secs as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Ensures low update intervals are clamped to prevent request floods.
    fn normalize_interval_clamps_small_values() {
        assert_eq!(normalize_interval(1), 60);
        assert_eq!(normalize_interval(59), 60);
    }

    #[test]
    /// Ensures valid update intervals are preserved unchanged.
    fn normalize_interval_keeps_valid_values() {
        assert_eq!(normalize_interval(60), 60);
        assert_eq!(normalize_interval(3600), 3600);
    }
}
