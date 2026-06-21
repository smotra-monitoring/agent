use crate::error::{Error, Result};
use std::path::Path;
use std::process::Command;

pub fn replace_binary_and_restart(new_binary_path: &Path) -> Result<()> {
    self_replace::self_replace(new_binary_path)
        .map_err(|e| Error::Unknown(format!("self-replace failed: {}", e)))?;

    trigger_restart()
}

pub fn trigger_restart() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let current_exe = std::env::current_exe().map_err(|e| {
            Error::Unknown(format!("failed to resolve current executable path: {}", e))
        })?;

        let exe = current_exe.to_string_lossy().to_string();
        let script = format!("choice /t 2 /d y > nul & start \"\" \"{}\"", exe);

        Command::new("cmd")
            .args(["/C", &script])
            .spawn()
            .map_err(|e| Error::Unknown(format!("failed to spawn restarter: {}", e)))?;
    }

    #[cfg(target_os = "macos")]
    {
        if super::environment::is_managed_by_launchd() {
            // simply exit and let launchd restart us
            std::process::exit(0);
        } else {
            // untested behavior
            let current_exe = std::env::current_exe().map_err(|e| {
                Error::Unknown(format!("failed to resolve current executable path: {}", e))
            })?;

            Command::new("open")
                .arg(current_exe)
                .spawn()
                .map_err(|e| Error::Unknown(format!("failed to spawn restarter: {}", e)))?;
        }
    }

    #[cfg(target_os = "linux")]
    {
        if super::environment::is_managed_by_systemd() {
            // simply exit and let systemd restart us
            std::process::exit(0);
        } else {
            // untested behavior
            use std::process::Stdio;

            let current_exe = std::env::current_exe().map_err(|e| {
                Error::Unknown(format!("failed to resolve current executable path: {}", e))
            })?;

            Command::new(current_exe)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|e| Error::Unknown(format!("failed to spawn restarter: {}", e)))?;
        }
    }

    std::process::exit(0);
}
