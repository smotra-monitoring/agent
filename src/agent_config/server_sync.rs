//! Server-driven config sync: fetch the desired config, merge local-only fields, persist to file.
//!
//! The file watcher picks up the persisted file and applies it through the normal hot-reload path.

use std::path::Path;

use reqwest::Url;
use tracing::{info, warn};

use crate::error::{Error, Result};
use crate::http_client::build_client;
use crate::openapi::AgentConfig;

/// Fetch the server's configuration for this agent and persist it to `config_path`.
///
/// Returns the version that was written.
pub(crate) async fn sync_config_from_server(
    local: &AgentConfig,
    config_path: &Path,
) -> Result<i64> {
    let remote = fetch_configuration(local).await?;
    let merged = merge_server_config(local, remote)?;

    merged.save_to_file_secure(config_path).await?;
    info!(
        "Server configuration (version {} -> {}) saved to {}",
        local.version,
        merged.version,
        config_path.display()
    );

    Ok(merged.version)
}

async fn fetch_configuration(local: &AgentConfig) -> Result<AgentConfig> {
    let client = build_client(local.server.timeout(), local.server.verify_tls)?;
    let url = format!(
        "{}/agent/{}/configuration",
        local.server.url, local.agent_id
    );

    let mut request = client.get(&url);
    if let Some(api_key) = &local.server.api_key {
        request = request.header("X-Agent-API-Key", api_key);
    }

    let response = request.send().await?;

    match response.status().as_u16() {
        200 => Ok(response.json::<AgentConfig>().await?),
        401 => {
            warn!("Configuration fetch rejected: Unauthorized (invalid API key)");
            Err(Error::Authentication(
                "Invalid API key for configuration fetch".to_string(),
            ))
        }
        status => {
            let body = response.text().await.unwrap_or_default();
            Err(Error::Network(format!(
                "Configuration fetch failed with status {}: {}",
                status, body
            )))
        }
    }
}

/// Apply server config on top of local state, keeping fields the server cannot know.
///
/// `api_key` is kept because the server only stores its digest; `cache_dir` is a host path.
/// Configs that lower transport security (https -> non-https, `verify_tls` true -> false) are rejected.
fn merge_server_config(local: &AgentConfig, mut remote: AgentConfig) -> Result<AgentConfig> {
    if remote.agent_id != local.agent_id {
        warn!(
            "Server config agent_id mismatch: expected {}, got {}",
            local.agent_id, remote.agent_id
        );
        return Err(Error::Config(format!(
            "server config is for agent {} but this agent is {}",
            remote.agent_id, local.agent_id
        )));
    }

    if remote.version <= local.version {
        warn!(
            "Server config version is older or the same: local {}, remote {}",
            local.version, remote.version
        );
        return Err(Error::Config(format!(
            "server config version {} is older or the same as local version {}",
            remote.version, local.version
        )));
    }

    reject_scheme_downgrade(local, &remote)?;
    reject_tls_verification_downgrade(local, &remote)?;

    remote.server.api_key = local.server.api_key.clone();
    remote.storage.cache_dir = local.storage.cache_dir.clone();

    remote.validate()?;
    Ok(remote)
}

fn reject_scheme_downgrade(local: &AgentConfig, remote: &AgentConfig) -> Result<()> {
    // An unparsable local URL is reported by local validation, not here.
    let Ok(local_url) = Url::parse(&local.server.url) else {
        return Ok(());
    };
    if local_url.scheme() != "https" {
        return Ok(());
    }

    let remote_url = Url::parse(&remote.server.url).map_err(|e| {
        Error::Config(format!(
            "server config has an invalid server URL '{}': {}",
            remote.server.url, e
        ))
    })?;
    if remote_url.scheme() != "https" {
        warn!(
            "Rejecting server config: server URL downgrade {} -> {}",
            local.server.url, remote.server.url
        );
        return Err(Error::Config(format!(
            "server config lowers security by changing server URL from {} to {}",
            local.server.url, remote.server.url
        )));
    }

    Ok(())
}

fn reject_tls_verification_downgrade(local: &AgentConfig, remote: &AgentConfig) -> Result<()> {
    if local.server.verify_tls && !remote.server.verify_tls {
        warn!("Rejecting server config: it disables TLS certificate verification");
        return Err(Error::Config(
            "server config lowers security by disabling TLS certificate verification".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::user_agent;
    use crate::openapi::{ServerConfig, StorageConfig};
    use tempfile::NamedTempFile;
    use uuid::Uuid;

    fn local_config(server_url: &str) -> AgentConfig {
        AgentConfig {
            version: 1,
            agent_id: Uuid::now_v7(),
            agent_name: "local".to_string(),
            server: ServerConfig {
                url: server_url.to_string(),
                api_key: Some("sk_local".to_string()),
                ..Default::default()
            },
            storage: StorageConfig {
                cache_dir: "/var/local/cache".to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// A newer config as the server would send it: no api_key, server-side cache_dir.
    fn remote_config(local: &AgentConfig, version: i64) -> AgentConfig {
        let mut remote = local.clone();
        remote.version = version;
        remote.agent_name = "from-server".to_string();
        remote.server.api_key = None;
        remote.storage.cache_dir = "./server-cache".to_string();
        remote
    }

    mod merge_server_config_tests {
        use super::*;

        #[test]
        fn keeps_api_key_and_cache_dir_and_takes_the_rest() {
            let mut local = local_config("https://old.example.com");
            local.server.verify_tls = false;
            let mut remote = remote_config(&local, 2);
            remote.server.url = "https://new.example.com".to_string();
            remote.self_upgrade.enabled = true;

            let merged = merge_server_config(&local, remote).unwrap();

            assert_eq!(merged.server.api_key.as_deref(), Some("sk_local"));
            assert_eq!(merged.storage.cache_dir, "/var/local/cache");
            assert_eq!(merged.version, 2);
            assert_eq!(merged.agent_name, "from-server");
            assert_eq!(merged.server.url, "https://new.example.com");
            assert!(merged.self_upgrade.enabled);
        }

        #[test]
        fn rejects_https_to_http_downgrade() {
            let local = local_config("https://x.example.com");
            let mut remote = remote_config(&local, 2);
            remote.server.url = "http://x.example.com".to_string();

            assert!(merge_server_config(&local, remote).is_err());
        }

        #[test]
        fn rejects_https_to_other_scheme() {
            let local = local_config("https://x.example.com");
            let mut remote = remote_config(&local, 2);
            remote.server.url = "ftp://x.example.com".to_string();

            assert!(merge_server_config(&local, remote).is_err());
        }

        #[test]
        fn rejects_downgrade_with_uppercase_scheme_in_remote() {
            let local = local_config("HTTPS://x.example.com");
            let mut remote = remote_config(&local, 2);
            remote.server.url = "HTTP://x.example.com".to_string();

            assert!(merge_server_config(&local, remote).is_err());
        }

        #[test]
        fn rejects_unparsable_remote_url_when_local_is_https() {
            let local = local_config("https://x.example.com");
            let mut remote = remote_config(&local, 2);
            remote.server.url = "not a url".to_string();

            assert!(merge_server_config(&local, remote).is_err());
        }

        #[test]
        fn allows_http_to_https_upgrade() {
            let local = local_config("http://x.example.com");
            let mut remote = remote_config(&local, 2);
            remote.server.url = "https://x.example.com".to_string();

            let merged = merge_server_config(&local, remote).unwrap();

            assert_eq!(merged.server.url, "https://x.example.com");
        }

        #[test]
        fn allows_http_to_http_change() {
            let local = local_config("http://x.example.com");
            let mut remote = remote_config(&local, 2);
            remote.server.url = "http://y.example.com".to_string();

            let merged = merge_server_config(&local, remote).unwrap();

            assert_eq!(merged.server.url, "http://y.example.com");
        }

        #[test]
        fn rejects_verify_tls_true_to_false() {
            let local = local_config("https://x.example.com");
            assert!(local.server.verify_tls);
            let mut remote = remote_config(&local, 2);
            remote.server.verify_tls = false;

            assert!(merge_server_config(&local, remote).is_err());
        }

        #[test]
        fn allows_verify_tls_false_to_true() {
            let mut local = local_config("https://x.example.com");
            local.server.verify_tls = false;
            let mut remote = remote_config(&local, 2);
            remote.server.verify_tls = true;

            let merged = merge_server_config(&local, remote).unwrap();

            assert!(merged.server.verify_tls);
        }

        #[test]
        fn rejects_agent_id_mismatch() {
            let local = local_config("https://x.example.com");
            let mut remote = remote_config(&local, 2);
            remote.agent_id = Uuid::now_v7();

            assert!(merge_server_config(&local, remote).is_err());
        }

        #[test]
        fn rejects_equal_and_lower_versions() {
            let local = local_config("https://x.example.com");

            assert!(merge_server_config(&local, remote_config(&local, 1)).is_err());
            assert!(merge_server_config(&local, remote_config(&local, 0)).is_err());
        }

        #[test]
        fn rejects_invalid_remote_config() {
            let local = local_config("https://x.example.com");
            let mut remote = remote_config(&local, 2);
            remote.server.heartbeat_interval_secs = 10;

            assert!(merge_server_config(&local, remote).is_err());
        }
    }

    mod fetch_configuration_tests {
        use super::*;

        #[tokio::test]
        async fn returns_config_on_200() {
            let mut server = mockito::Server::new_async().await;
            let local = local_config(&server.url());
            let remote = remote_config(&local, 2);
            let mock = server
                .mock(
                    "GET",
                    format!("/agent/{}/configuration", local.agent_id).as_str(),
                )
                .match_header("user-agent", user_agent())
                .match_header("x-agent-api-key", "sk_local")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(serde_json::to_string(&remote).unwrap())
                .create_async()
                .await;

            let fetched = fetch_configuration(&local).await.unwrap();

            assert_eq!(fetched.version, 2);
            assert_eq!(fetched.agent_name, "from-server");
            mock.assert_async().await;
        }

        #[tokio::test]
        async fn maps_401_to_authentication_error() {
            let mut server = mockito::Server::new_async().await;
            let local = local_config(&server.url());
            server
                .mock(
                    "GET",
                    format!("/agent/{}/configuration", local.agent_id).as_str(),
                )
                .with_status(401)
                .create_async()
                .await;

            let err = fetch_configuration(&local).await.unwrap_err();

            assert!(matches!(err, Error::Authentication(_)), "got {:?}", err);
        }

        #[tokio::test]
        async fn maps_404_501_and_503_to_network_error() {
            for status in [404, 501, 503] {
                let mut server = mockito::Server::new_async().await;
                let local = local_config(&server.url());
                server
                    .mock(
                        "GET",
                        format!("/agent/{}/configuration", local.agent_id).as_str(),
                    )
                    .with_status(status)
                    .create_async()
                    .await;

                let err = fetch_configuration(&local).await.unwrap_err();

                assert!(
                    matches!(err, Error::Network(ref m) if m.contains(&status.to_string())),
                    "status {} gave {:?}",
                    status,
                    err
                );
            }
        }

        #[tokio::test]
        async fn fails_on_malformed_body() {
            let mut server = mockito::Server::new_async().await;
            let local = local_config(&server.url());
            server
                .mock(
                    "GET",
                    format!("/agent/{}/configuration", local.agent_id).as_str(),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body("{not json")
                .create_async()
                .await;

            assert!(fetch_configuration(&local).await.is_err());
        }

        #[tokio::test]
        async fn fails_when_server_unreachable() {
            let local = local_config("http://127.0.0.1:1");

            assert!(fetch_configuration(&local).await.is_err());
        }
    }

    mod sync_config_from_server_tests {
        use super::*;

        #[tokio::test]
        async fn persists_merged_config_with_secure_permissions() {
            let mut server = mockito::Server::new_async().await;
            let local = local_config(&server.url());
            let remote = remote_config(&local, 2);
            server
                .mock(
                    "GET",
                    format!("/agent/{}/configuration", local.agent_id).as_str(),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(serde_json::to_string(&remote).unwrap())
                .create_async()
                .await;
            let file = NamedTempFile::new().unwrap();

            let version = sync_config_from_server(&local, file.path()).await.unwrap();

            assert_eq!(version, 2);
            let saved = AgentConfig::from_file(file.path()).unwrap();
            assert_eq!(saved.version, 2);
            assert_eq!(saved.agent_name, "from-server");
            assert_eq!(saved.server.api_key.as_deref(), Some("sk_local"));
            assert_eq!(saved.storage.cache_dir, "/var/local/cache");

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = std::fs::metadata(file.path()).unwrap().permissions().mode();
                assert_eq!(mode & 0o777, 0o600);
            }
        }

        #[tokio::test]
        async fn leaves_file_untouched_when_merge_is_rejected() {
            let mut server = mockito::Server::new_async().await;
            let local = local_config(&server.url());
            server
                .mock(
                    "GET",
                    format!("/agent/{}/configuration", local.agent_id).as_str(),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(serde_json::to_string(&remote_config(&local, 1)).unwrap())
                .create_async()
                .await;
            let file = NamedTempFile::new().unwrap();
            std::fs::write(file.path(), "original").unwrap();

            assert!(sync_config_from_server(&local, file.path()).await.is_err());

            assert_eq!(std::fs::read_to_string(file.path()).unwrap(), "original");
        }

        #[tokio::test]
        async fn leaves_file_untouched_when_security_is_downgraded() {
            let mut server = mockito::Server::new_async().await;
            let local = local_config(&server.url());
            let mut remote = remote_config(&local, 2);
            remote.server.verify_tls = false;
            server
                .mock(
                    "GET",
                    format!("/agent/{}/configuration", local.agent_id).as_str(),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(serde_json::to_string(&remote).unwrap())
                .create_async()
                .await;
            let file = NamedTempFile::new().unwrap();
            std::fs::write(file.path(), "original").unwrap();

            assert!(sync_config_from_server(&local, file.path()).await.is_err());

            assert_eq!(std::fs::read_to_string(file.path()).unwrap(), "original");
        }
    }
}
