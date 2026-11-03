//! User-Agent string sent with every outgoing HTTP/HTTPS request.

use std::sync::OnceLock;

const PROJECT_URL: &str = "https://github.com/smotra-monitoring/agent";

/// `smotra-agent/<version> (+<project url>; <os>; <arch>)`, built once.
pub(crate) fn user_agent() -> &'static str {
    static USER_AGENT: OnceLock<String> = OnceLock::new();
    USER_AGENT.get_or_init(|| {
        format!(
            "smotra-agent/{} (+{}; {}; {})",
            env!("CARGO_PKG_VERSION"),
            PROJECT_URL,
            std::env::consts::OS,
            std::env::consts::ARCH,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_has_product_version_and_platform() {
        let ua = user_agent();
        assert!(ua.starts_with(&format!("smotra-agent/{} (", env!("CARGO_PKG_VERSION"))));
        assert!(ua.contains(PROJECT_URL));
        assert!(ua.contains(std::env::consts::OS));
        assert!(ua.contains(std::env::consts::ARCH));
    }

    #[test]
    fn user_agent_is_valid_header_value() {
        assert!(reqwest::header::HeaderValue::from_str(user_agent()).is_ok());
    }
}
