use crate::error::{Error, Result};

/// Parses a GitHub repository URL and returns `(owner, repo)`.
///
/// Accepts `https://github.com/owner/repo` and `http://github.com/owner/repo`,
/// with or without a trailing `.git` suffix or trailing slash.
/// Rejects any URL whose host is not exactly `github.com`.
pub(super) fn parse_github_url(url: &str) -> Result<(String, String)> {
    let url = url.trim().trim_end_matches('/');

    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
        .ok_or_else(|| {
            Error::Config(format!(
                "Not a GitHub URL: '{}'. Expected 'https://github.com/owner/repo'",
                url
            ))
        })?;

    let mut parts = path.splitn(3, '/');
    let owner = parts.next().unwrap_or("").trim();
    let repo_raw = parts.next().unwrap_or("").trim();
    let repo = repo_raw.trim_end_matches(".git");

    if owner.is_empty() {
        return Err(Error::Config(format!(
            "GitHub URL is missing the repository owner: '{}'. Expected 'https://github.com/owner/repo'",
            url
        )));
    }

    if repo.is_empty() {
        return Err(Error::Config(format!(
            "GitHub URL is missing the repository name: '{}'. Expected 'https://github.com/owner/repo'",
            url
        )));
    }

    Ok((owner.to_string(), repo.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_github_url_tests {
        use super::*;

        #[test]
        /// Parses a standard HTTPS GitHub URL into owner and repo.
        fn parses_standard_https_url() {
            let (owner, repo) = parse_github_url("https://github.com/owner/repo").unwrap();
            assert_eq!(owner, "owner");
            assert_eq!(repo, "repo");
        }

        #[test]
        /// Strips trailing slash before parsing.
        fn strips_trailing_slash() {
            let (owner, repo) = parse_github_url("https://github.com/owner/repo/").unwrap();
            assert_eq!(owner, "owner");
            assert_eq!(repo, "repo");
        }

        #[test]
        /// Strips .git suffix from the repository name.
        fn strips_git_suffix() {
            let (owner, repo) = parse_github_url("https://github.com/owner/repo.git").unwrap();
            assert_eq!(owner, "owner");
            assert_eq!(repo, "repo");
        }

        #[test]
        /// Accepts http:// scheme as well as https://.
        fn accepts_http_scheme() {
            let (owner, repo) = parse_github_url("http://github.com/owner/repo").unwrap();
            assert_eq!(owner, "owner");
            assert_eq!(repo, "repo");
        }

        #[test]
        /// Returns an error for non-GitHub URLs.
        fn rejects_non_github_url() {
            let result = parse_github_url("https://gitlab.com/owner/repo");
            assert!(result.is_err(), "non-GitHub URL should be rejected");
        }

        #[test]
        /// Returns an error for URLs that look like GitHub but have a different host.
        fn rejects_fake_github_host() {
            let result = parse_github_url("https://notgithub.com/owner/repo");
            assert!(result.is_err(), "lookalike host should be rejected");
        }

        #[test]
        /// Returns an error when the URL is missing both owner and repo.
        fn rejects_url_without_owner_and_repo() {
            let result = parse_github_url("https://github.com/");
            assert!(result.is_err(), "URL without owner/repo should be rejected");
        }

        #[test]
        /// Returns an error when the URL has an owner but no repo.
        fn rejects_url_without_repo() {
            let result = parse_github_url("https://github.com/owner");
            assert!(result.is_err(), "URL without repo should be rejected");
        }

        #[test]
        /// Extra path segments beyond owner/repo are ignored.
        fn ignores_extra_path_segments() {
            let (owner, repo) =
                parse_github_url("https://github.com/owner/repo/tree/main").unwrap();
            assert_eq!(owner, "owner");
            assert_eq!(repo, "repo");
        }
    }
}
