use crate::error::{Error, Result};
use octocrab::Octocrab;
use semver::Version;

use super::github::parse_github_url;

pub async fn fetch_latest_version(octocrab: &Octocrab, repo_url: &str) -> Result<Version> {
    let (owner, repo) = parse_github_url(repo_url)?;

    let release = octocrab
        .repos(&owner, &repo)
        .releases()
        .get_latest()
        .await
        .map_err(|e| Error::GithubApi(format!("Failed to fetch latest release: {}", e)))?;

    let normalized = release.tag_name.trim().trim_start_matches('v');
    Version::parse(normalized).map_err(|e| {
        Error::Config(format!(
            "Invalid version in GitHub release tag '{}': {}",
            release.tag_name, e
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
