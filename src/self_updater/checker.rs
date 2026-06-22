use crate::error::{Error, Result};
use semver::Version;

pub async fn fetch_latest_version(client: &reqwest::Client, check_url: &str) -> Result<Version> {
    let base = check_url.trim_end_matches('/');
    let version_url = format!("{}/releases/latest/version.txt", base);

    let response = client
        .get(&version_url)
        .send()
        .await
        .map_err(|e| Error::Network(format!("Failed to fetch latest version: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::Network(format!(
            "Version endpoint returned HTTP {}",
            response.status()
        )));
    }

    let raw = response
        .text()
        .await
        .map_err(|e| Error::Network(format!("Failed to read version response: {}", e)))?;

    let normalized = raw.trim().trim_start_matches('v');
    Version::parse(normalized).map_err(|e| {
        Error::Config(format!(
            "Invalid latest version fetched from server '{}': {}",
            raw.trim(),
            e
        ))
    })
}

pub fn is_newer_than_current(latest: &Version) -> Result<bool> {
    let current = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|e| {
        Error::Config(format!(
            "Invalid current package version '{}': {}",
            env!("CARGO_PKG_VERSION"),
            e
        ))
    })?;

    Ok(latest > &current)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod is_newer_than_current_tests {
        use super::*;

        #[test]
        /// Verifies that a clearly newer version is detected as upgrade candidate.
        fn detects_newer_version() {
            let latest = Version::parse("99.0.0").unwrap();
            let newer = is_newer_than_current(&latest).unwrap();
            assert!(newer, "99.0.0 should be newer than current package version");
        }

        #[test]
        /// Verifies that older versions are not treated as upgrade candidates.
        fn rejects_older_version() {
            let latest = Version::parse("0.0.1").unwrap();
            let newer = is_newer_than_current(&latest).unwrap();
            assert!(
                !newer,
                "0.0.1 should not be newer than current package version"
            );
        }
    }
}
