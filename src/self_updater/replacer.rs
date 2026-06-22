use crate::error::{Error, Result};
use std::path::Path;
use std::process::Command;

pub fn replace_binary_and_restart(new_binary_path: &Path) -> Result<()> {
    // Resolve the absolute path of the current executable
    // This is necessary because self_replace may move the new binary to another location,
    //
    // In Linux restarting another process may fail - the file that was running has been renamed or unlinked to make room for the new one.
    // Because the process is still holding the "old" binary's memory map, the kernel reports that path as "smotra (deleted)"
    // With actual postfix " (deleted)" which is not the same as the path of the new binary, so we need to resolve the absolute path before replacing.
    let target_path = std::env::current_exe()
        .map_err(|_| Error::SelfUpgrade("Failed to resolve current exe".into()))?;
    let absolute_target = std::fs::canonicalize(&target_path).unwrap_or(target_path);

    self_replace::self_replace(new_binary_path)
        .map_err(|e| Error::SelfUpgrade(format!("self-replace failed: {}", e)))?;

    trigger_restart(&absolute_target)
}

pub fn trigger_restart(current_exe: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let exe = current_exe.to_string_lossy().to_string();
        let script = format!("choice /t 2 /d y > nul & start \"\" \"{}\"", exe);

        Command::new("cmd")
            .args(["/C", &script])
            .spawn()
            .map_err(|e| Error::SelfUpgrade(format!("failed to spawn restarter: {}", e)))?;
    }

    #[cfg(target_os = "macos")]
    {
        if super::environment::is_managed_by_launchd() {
            // simply exit and let launchd restart us
            std::process::exit(0);
        } else {
            // untested behavior

            Command::new("open")
                .arg(current_exe)
                .spawn()
                .map_err(|e| Error::SelfUpgrade(format!("failed to spawn restarter: {}", e)))?;
        }
    }

    #[cfg(target_os = "linux")]
    {
        if super::environment::is_managed_by_systemd() {
            // simply exit and let systemd restart us
            std::process::exit(0);
        } else {
            // untested behavior
            use std::os::unix::process::CommandExt; // Import this trait

            println!("Spawning restarter: {}", current_exe.display());

            let err = Command::new(current_exe)
                .args(std::env::args().skip(1))
                .exec();

            // If .exec() returns, it means there was an error
            unreachable!("Restarter exec failed: {}", err);
        }
    }
}
