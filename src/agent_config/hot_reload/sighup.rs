//! SIGHUP signal handling for config reload
//!
//! Provides Unix-specific signal handling to trigger config reloads via SIGHUP.
//! On non-Unix platforms, this is a no-op.

use tokio::sync::{broadcast, mpsc};
use tracing::info;

use crate::error::{Error, Result};

use super::orchestrator::ReloadTrigger;

/// Handle SIGHUP signal and trigger config reload
///
/// Unix-only function that listens for SIGHUP and sends reload triggers.
///
/// # Arguments
///
/// * `reload_tx` - Channel to send reload triggers
/// * `shutdown_rx` - Channel to receive shutdown signals
#[cfg(unix)]
pub async fn handle_sighup(
    reload_tx: mpsc::UnboundedSender<ReloadTrigger>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sighup = signal(SignalKind::hangup())
        .map_err(|e| Error::SigHup(format!("Failed to setup SIGHUP handler: {}", e)))?;

    info!("SIGHUP handler started");

    loop {
        tokio::select! {
            _ = sighup.recv() => {
                info!("SIGHUP received, triggering config reload");
                reload_tx.send(ReloadTrigger::Signal)
                    .map_err(|e| Error::SigHup(format!("Failed to send reload trigger: {}", e)))?;
            }
            _ = shutdown_rx.recv() => {
                info!("SIGHUP handler shutting down");
                break;
            }
        }
    }

    Ok(())
}

/// Handle SIGHUP signal (no-op on non-Unix systems)
#[cfg(not(unix))]
pub async fn handle_sighup(
    _reload_tx: mpsc::UnboundedSender<ReloadTrigger>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    warn!("SIGHUP handler not supported on this platform");
    let _ = shutdown_rx.recv().await;
    Ok(())
}
