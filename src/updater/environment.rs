use std::path::Path;

pub fn is_containerized() -> bool {
    Path::new("/.dockerenv").exists() || std::env::var("CONTAINER").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Ensures the helper reports true when CONTAINER env var is present.
    fn detects_container_env_var() {
        let old = std::env::var("CONTAINER").ok();

        std::env::set_var("CONTAINER", "1");
        let result = is_containerized();
        assert!(result, "expected container detection to be true");

        if let Some(prev) = old {
            std::env::set_var("CONTAINER", prev);
        } else {
            std::env::remove_var("CONTAINER");
        }
    }
}
