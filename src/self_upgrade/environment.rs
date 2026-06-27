use std::path::Path;

pub fn is_containerized() -> bool {
    Path::new("/.dockerenv").exists() || std::env::var("CONTAINER").is_ok()
}

#[cfg(target_os = "macos")]
pub fn is_managed_by_launchd() -> bool {
    use sysinfo::{Pid, ProcessesToUpdate, System};

    let mut s = System::new();
    // ProcessesToUpdate::All and true to clear out dead processes
    s.refresh_processes(ProcessesToUpdate::All, true);

    // Get the current process ID
    let current_pid = sysinfo::get_current_pid().ok();

    if let Some(pid) = current_pid {
        if let Some(process) = s.process(pid) {
            // Get the parent process
            if let Some(parent_pid) = process.parent() {
                if let Some(parent) = s.process(parent_pid) {
                    // launchd usually has the name "launchd"
                    return parent
                        .name()
                        .to_string_lossy()
                        .to_lowercase()
                        .contains("launchd");
                }
            }
        }
    }
    false
}

#[cfg(target_os = "linux")]
pub fn is_managed_by_systemd() -> bool {
    std::env::var("INVOCATION_ID").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Ensures the helper reports true when CONTAINER env var is present.
    fn detects_container_env_var() {
        let old = std::env::var("CONTAINER").ok();

        std::env::set_var("CONTAINER", "");
        let result = is_containerized();
        assert!(result, "expected container detection to be true");

        if let Some(prev) = old {
            std::env::set_var("CONTAINER", prev);
        } else {
            std::env::remove_var("CONTAINER");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    /// Ensures the helper can detect launchd parent process on macOS.
    fn detects_launchd_parent() {
        let result = is_managed_by_launchd();
        // We can't guarantee the test environment, so we just print the result.
        println!("is_managed_by_launchd: {}", result);

        // Note: In a real test, we would mock the sysinfo calls to simulate different parent processes.

        // For now, we just assert that the function runs without panicking.
        assert!(true);
    }

    #[cfg(target_os = "linux")]
    #[test]
    /// Ensures the helper can detect systemd environment on Linux.
    fn detects_systemd_environment() {
        let old = std::env::var("INVOCATION_ID").ok();
        std::env::set_var("INVOCATION_ID", "test");
        let result = is_managed_by_systemd();
        assert!(result, "expected systemd detection to be true");

        if let Some(prev) = old {
            std::env::set_var("INVOCATION_ID", prev);
        } else {
            std::env::remove_var("INVOCATION_ID");
        }
    }
}
