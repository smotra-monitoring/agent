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
        if is_managed_by_launchd() {
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
        let _ = Command::new("systemctl")
            .args(["restart", "smotra"])
            .spawn();
    }

    std::process::exit(0);
}

#[cfg(target_os = "macos")]
pub fn is_managed_by_launchd() -> bool {
    use sysinfo::{Pid, System};

    let mut s = System::new();
    s.refresh_processes();

    // Get the current process ID
    let current_pid = sysinfo::get_current_pid().ok();

    if let Some(pid) = current_pid {
        if let Some(process) = s.process(pid) {
            // Get the parent process
            if let Some(parent_pid) = process.parent() {
                if let Some(parent) = s.process(parent_pid) {
                    // launchd usually has the name "launchd"
                    return parent.name().to_lowercase().contains("launchd");
                }
            }
        }
    }
    false
}
