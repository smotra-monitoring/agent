//! Server-driven config sync: fetch the stored config patch, merge it into the local config, persist to file.
//!
//! The server returns a JSON Merge Patch (RFC 7396); the effective config is
//! `Default` -> local config file -> server patch.
//! The file watcher picks up the persisted
//! file and applies it through the normal hot-reload path.

use std::path::Path;

use reqwest::Url;
use serde_json::Value;
use tracing::{info, warn};

use super::merge_patch::{layer_over_defaults, merge_patch, reject_unknown_fields};
use crate::error::{Error, Result};
use crate::http_client::build_client;
use crate::openapi::{AgentConfig, AgentConfigPatch};

/// Config paths the patch must not touch: identity comes from the envelope, the rest is host-owned.
const PROTECTED_KEYS: &[&[&str]] = &[
    &["agent_id"],
    &["agent_name"],
    &["version"],
    &["server", "api_key"],
    &["storage", "cache_dir"],
];

/// Fetch the server's configuration for this agent and persist it to `config_path`.
///
/// Returns the version that was written.
pub(crate) async fn sync_config_from_server(
    local: &AgentConfig,
    config_path: &Path,
) -> Result<i64> {
    let envelope = fetch_configuration(local).await?;
    let merged = merge_server_config(local, &envelope)?;

    merged.save_to_file_secure(config_path).await?;
    info!(
        "Server configuration (version {} -> {}) saved to {}",
        local.version,
        merged.version,
        config_path.display()
    );

    Ok(merged.version)
}

async fn fetch_configuration(local: &AgentConfig) -> Result<AgentConfigPatch> {
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
        200 => Ok(response.json::<AgentConfigPatch>().await?),
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

/// Apply the server's config patch on top of the local config.
///
/// Any problem with the envelope or the patch rejects the whole config; nothing is partially applied.
/// Configs that lower transport security (https -> non-https, `verify_tls` true -> false) are rejected.
fn merge_server_config(local: &AgentConfig, envelope: &AgentConfigPatch) -> Result<AgentConfig> {
    check_identity(local, envelope)?;
    check_version(local, envelope)?;

    let patch = Value::Object(envelope.base_config.clone().into_iter().collect());
    reject_protected_keys(&patch)?;
    reject_unknown_fields(&patch)?;

    let effective = build_effective(local, &patch)?;
    let merged = into_config(effective, envelope)?;

    merged.validate()?;
    reject_scheme_downgrade(local, &merged)?;
    reject_tls_verification_downgrade(local, &merged)?;

    Ok(merged)
}

fn check_identity(local: &AgentConfig, envelope: &AgentConfigPatch) -> Result<()> {
    if envelope.id != local.agent_id {
        warn!(
            "Server config agent_id mismatch: expected {}, got {}",
            local.agent_id, envelope.id
        );
        return Err(Error::Config(format!(
            "server config is for agent {} but this agent is {}",
            envelope.id, local.agent_id
        )));
    }

    Ok(())
}

fn check_version(local: &AgentConfig, envelope: &AgentConfigPatch) -> Result<()> {
    if envelope.config_version_desired <= local.version {
        warn!(
            "Server config version is older or the same: local {}, remote {}",
            local.version, envelope.config_version_desired
        );
        return Err(Error::Config(format!(
            "server config version {} is older or the same as local version {}",
            envelope.config_version_desired, local.version
        )));
    }

    Ok(())
}

fn reject_protected_keys(patch: &Value) -> Result<()> {
    match PROTECTED_KEYS.iter().find(|path| touches_path(patch, path)) {
        Some(path) => Err(Error::Config(format!(
            "server config must not set '{}'",
            path.join(".")
        ))),
        None => Ok(()),
    }
}

/// True if the patch sets or removes `path`, including by nulling one of its parent objects.
fn touches_path(patch: &Value, path: &[&str]) -> bool {
    let Some((key, rest)) = path.split_first() else {
        return true;
    };

    match patch.get(key) {
        None => false,
        Some(Value::Null) => true,
        Some(child) => touches_path(child, rest),
    }
}

/// Default -> config-file -> server patch
///
/// `local` is already fully populated, so defaults are layered again here to
/// restore keys the patch nulls out (RFC 7396 removal => default applies).
fn build_effective(local: &AgentConfig, patch: &Value) -> Result<Value> {
    let mut overrides = serde_json::to_value(local)?;
    merge_patch(&mut overrides, patch);
    layer_over_defaults(&overrides)
}

/// 1. Converts from the JSON::Value into an `AgentConfig`.
/// 2. Copy agent_name and config_version from the envelope into the effective config.
fn into_config(effective: Value, envelope: &AgentConfigPatch) -> Result<AgentConfig> {
    let mut config: AgentConfig = serde_json::from_value(effective)
        .map_err(|e| Error::Config(format!("server config is not a valid agent config: {}", e)))?;
    config.agent_name = envelope.name.clone();
    config.version = envelope.config_version_desired;

    Ok(config)
}

fn reject_scheme_downgrade(local: &AgentConfig, remote: &AgentConfig) -> Result<()> {
    // An unparsable local URL is reported by local validation, not here.
    let Ok(local_url) = Url::parse(&local.server.url) else {
        warn!(
            "Local server URL '{}' is not valid, skipping scheme downgrade check",
            local.server.url
        );
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
            "Rejecting server config (version {}): server URL downgrade {} -> {}",
            remote.version, local.server.url, remote.server.url
        );
        return Err(Error::Config(format!(
            "server config (version {}) lowers security by changing server URL from {} to {}",
            remote.version, local.server.url, remote.server.url
        )));
    }

    Ok(())
}

fn reject_tls_verification_downgrade(local: &AgentConfig, remote: &AgentConfig) -> Result<()> {
    if local.server.verify_tls && !remote.server.verify_tls {
        warn!(
            "Rejecting server config (version {}): it disables TLS certificate verification",
            remote.version
        );
        return Err(Error::Config(format!(
            "server config (version {}) lowers security by disabling TLS certificate verification",
            remote.version
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::user_agent;
    use crate::openapi::{ServerConfig, StorageConfig};
    use serde_json::json;
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

    /// The server's response for `local`: the stored patch as is, plus identity and version.
    fn envelope(local: &AgentConfig, version: i64, base_config: Value) -> AgentConfigPatch {
        AgentConfigPatch {
            id: local.agent_id,
            workspace_id: Uuid::now_v7(),
            name: "from-server".to_string(),
            config_version_desired: version,
            base_config: serde_json::from_value(base_config).unwrap(),
        }
    }

    fn merge(local: &AgentConfig, base_config: Value) -> Result<AgentConfig> {
        merge_server_config(local, &envelope(local, 2, base_config))
    }

    mod check_identity_tests {
        use super::*;

        #[test]
        fn accepts_matching_id_and_rejects_mismatch() {
            let local = local_config("https://x.example.com");
            let mut remote = envelope(&local, 2, json!({}));

            assert!(check_identity(&local, &remote).is_ok());

            remote.id = Uuid::now_v7();
            assert!(check_identity(&local, &remote).is_err());
        }
    }

    mod check_version_tests {
        use super::*;

        #[test]
        fn accepts_only_a_newer_version() {
            let local = local_config("https://x.example.com");

            assert!(check_version(&local, &envelope(&local, 2, json!({}))).is_ok());
            assert!(check_version(&local, &envelope(&local, 1, json!({}))).is_err());
            assert!(check_version(&local, &envelope(&local, 0, json!({}))).is_err());
        }
    }

    mod reject_protected_keys_tests {
        use super::*;

        #[test]
        fn rejects_each_protected_key() {
            let patches = [
                json!({"agent_id": "0197f96c-b278-7f64-a32f-dae3cabe1ff0"}),
                json!({"agent_name": "x"}),
                json!({"version": 9}),
                json!({"server": {"api_key": "sk_other"}}),
                json!({"storage": {"cache_dir": "/tmp"}}),
            ];

            for patch in patches {
                assert!(reject_protected_keys(&patch).is_err(), "patch {patch}");
            }
        }

        #[test]
        fn rejects_null_on_a_protected_key_or_its_parent() {
            assert!(reject_protected_keys(&json!({"server": {"api_key": null}})).is_err());
            assert!(reject_protected_keys(&json!({"server": null})).is_err());
            assert!(reject_protected_keys(&json!({"storage": null})).is_err());
        }

        #[test]
        fn accepts_other_keys_of_the_same_sections() {
            let patch =
                json!({"server": {"url": "https://x"}, "storage": {"cache_enabled": false}});

            assert!(reject_protected_keys(&patch).is_ok());
        }
    }

    mod build_effective_tests {
        use super::*;

        #[test]
        fn patch_overrides_only_the_keys_it_names() {
            let mut local = local_config("https://x.example.com");
            local.monitoring.interval_secs = 10;
            local.monitoring.timeout_secs = 7;

            let effective =
                build_effective(&local, &json!({"monitoring": {"interval_secs": 30}})).unwrap();

            assert_eq!(effective["monitoring"]["interval_secs"], 30);
            assert_eq!(effective["monitoring"]["timeout_secs"], 7);
            assert_eq!(effective["server"]["api_key"], "sk_local");
        }

        #[test]
        fn null_resets_a_key_or_a_section_to_the_default() {
            let mut local = local_config("https://x.example.com");
            local.monitoring.interval_secs = 10;
            local.monitoring.timeout_secs = 7;
            local.storage.max_cached_results = 5;

            let key =
                build_effective(&local, &json!({"monitoring": {"timeout_secs": null}})).unwrap();
            let section = build_effective(&local, &json!({"monitoring": null})).unwrap();

            assert_eq!(key["monitoring"]["timeout_secs"], 1);
            assert_eq!(key["monitoring"]["interval_secs"], 10);
            assert_eq!(section["monitoring"]["interval_secs"], 60);
            assert_eq!(section["monitoring"]["timeout_secs"], 1);
            assert_eq!(section["storage"]["max_cached_results"], 5);
        }

        #[test]
        fn arrays_are_replaced_as_a_whole() {
            let mut local = local_config("https://x.example.com");
            local.tags = vec!["a".to_string(), "b".to_string()];

            let effective = build_effective(&local, &json!({"tags": ["c"]})).unwrap();

            assert_eq!(effective["tags"], json!(["c"]));
        }
    }

    mod into_config_tests {
        use super::*;

        #[test]
        fn takes_name_and_version_from_the_envelope() {
            let local = local_config("https://x.example.com");
            let effective = build_effective(&local, &json!({})).unwrap();

            let config = into_config(effective, &envelope(&local, 5, json!({}))).unwrap();

            assert_eq!(config.agent_name, "from-server");
            assert_eq!(config.version, 5);
            assert_eq!(config.agent_id, local.agent_id);
        }

        #[test]
        fn fails_on_a_value_of_the_wrong_type() {
            let local = local_config("https://x.example.com");
            let effective =
                build_effective(&local, &json!({"monitoring": {"interval_secs": "fast"}})).unwrap();

            assert!(into_config(effective, &envelope(&local, 2, json!({}))).is_err());
        }
    }

    mod merge_server_config_tests {
        use super::*;

        #[test]
        fn applies_a_partial_patch_and_keeps_local_only_fields() {
            let mut local = local_config("https://old.example.com");
            local.server.verify_tls = false;
            local.monitoring.timeout_secs = 7;

            let merged = merge(
                &local,
                json!({
                    "server": {"url": "https://new.example.com"},
                    "monitoring": {"interval_secs": 30},
                    "self_upgrade": {"enabled": true}
                }),
            )
            .unwrap();

            assert_eq!(merged.server.url, "https://new.example.com");
            assert_eq!(merged.monitoring.interval_secs, 30);
            assert_eq!(merged.monitoring.timeout_secs, 7);
            assert!(merged.self_upgrade.enabled);
            assert_eq!(merged.server.api_key.as_deref(), Some("sk_local"));
            assert_eq!(merged.storage.cache_dir, "/var/local/cache");
            assert_eq!(merged.agent_name, "from-server");
            assert_eq!(merged.version, 2);
        }

        #[test]
        fn empty_patch_only_bumps_name_and_version() {
            let local = local_config("https://x.example.com");

            let merged = merge(&local, json!({})).unwrap();

            assert_eq!(merged.version, 2);
            assert_eq!(merged.server.url, local.server.url);
            assert_eq!(
                merged.monitoring.interval_secs,
                local.monitoring.interval_secs
            );
            assert_eq!(merged.agent_name, "from-server");
        }

        #[test]
        fn rejects_unknown_fields_and_wrong_types() {
            let local = local_config("https://x.example.com");

            assert!(merge(&local, json!({"bogus": 1})).is_err());
            assert!(merge(&local, json!({"monitoring": {"intervl_secs": 1}})).is_err());
            assert!(merge(&local, json!({"monitoring": {"interval_secs": "x"}})).is_err());
        }

        #[test]
        fn rejects_protected_keys() {
            let local = local_config("https://x.example.com");

            assert!(merge(&local, json!({"server": {"api_key": "sk_other"}})).is_err());
            assert!(merge(&local, json!({"server": null})).is_err());
        }

        #[test]
        fn rejects_agent_id_mismatch_and_stale_version() {
            let local = local_config("https://x.example.com");
            let mut wrong_id = envelope(&local, 2, json!({}));
            wrong_id.id = Uuid::now_v7();

            assert!(merge_server_config(&local, &wrong_id).is_err());
            assert!(merge_server_config(&local, &envelope(&local, 1, json!({}))).is_err());
        }

        #[test]
        fn rejects_a_patch_that_fails_validation() {
            let local = local_config("https://x.example.com");

            assert!(merge(&local, json!({"server": {"heartbeat_interval_secs": 10}})).is_err());
        }

        #[test]
        fn rejects_https_to_non_https_downgrade() {
            let local = local_config("https://x.example.com");

            for url in ["http://x.example.com", "ftp://x.example.com", "not a url"] {
                assert!(
                    merge(&local, json!({"server": {"url": url}})).is_err(),
                    "url {url}"
                );
            }
        }

        #[test]
        fn rejects_downgrade_with_uppercase_scheme() {
            let local = local_config("HTTPS://x.example.com");

            assert!(merge(&local, json!({"server": {"url": "HTTP://x.example.com"}})).is_err());
        }

        #[test]
        fn allows_http_to_https_and_http_to_http_changes() {
            let local = local_config("http://x.example.com");

            let upgraded = merge(&local, json!({"server": {"url": "https://x.example.com"}}));
            let changed = merge(&local, json!({"server": {"url": "http://y.example.com"}}));

            assert_eq!(upgraded.unwrap().server.url, "https://x.example.com");
            assert_eq!(changed.unwrap().server.url, "http://y.example.com");
        }

        #[test]
        fn null_url_falls_back_to_the_default_https_url() {
            let local = local_config("http://x.example.com");

            let merged = merge(&local, json!({"server": {"url": null}})).unwrap();

            assert_eq!(merged.server.url, ServerConfig::default().url);
        }

        #[test]
        fn rejects_disabling_tls_verification() {
            let local = local_config("https://x.example.com");
            assert!(local.server.verify_tls);

            assert!(merge(&local, json!({"server": {"verify_tls": false}})).is_err());
        }

        #[test]
        fn allows_enabling_tls_verification() {
            let mut local = local_config("https://x.example.com");
            local.server.verify_tls = false;

            let merged = merge(&local, json!({"server": {"verify_tls": true}})).unwrap();

            assert!(merged.server.verify_tls);
        }
    }

    mod fetch_configuration_tests {
        use super::*;

        #[tokio::test]
        async fn returns_envelope_on_200() {
            let mut server = mockito::Server::new_async().await;
            let local = local_config(&server.url());
            let body = envelope(&local, 2, json!({"monitoring": {"interval_secs": 30}}));
            let mock = server
                .mock(
                    "GET",
                    format!("/agent/{}/configuration", local.agent_id).as_str(),
                )
                .match_header("user-agent", user_agent())
                .match_header("x-agent-api-key", "sk_local")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(serde_json::to_string(&body).unwrap())
                .create_async()
                .await;

            let fetched = fetch_configuration(&local).await.unwrap();

            assert_eq!(fetched.config_version_desired, 2);
            assert_eq!(fetched.name, "from-server");
            assert_eq!(fetched.base_config["monitoring"]["interval_secs"], 30);
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
        async fn fails_on_malformed_or_non_object_base_config() {
            for body in [
                "{not json".to_string(),
                format!(
                    r#"{{"id":"{0}","workspace_id":"{0}","name":"n","config_version_desired":2,"base_config":"{{}}"}}"#,
                    Uuid::now_v7()
                ),
            ] {
                let mut server = mockito::Server::new_async().await;
                let local = local_config(&server.url());
                server
                    .mock(
                        "GET",
                        format!("/agent/{}/configuration", local.agent_id).as_str(),
                    )
                    .with_status(200)
                    .with_header("content-type", "application/json")
                    .with_body(body.as_str())
                    .create_async()
                    .await;

                assert!(fetch_configuration(&local).await.is_err(), "body {body}");
            }
        }

        #[tokio::test]
        async fn fails_when_server_unreachable() {
            let local = local_config("http://127.0.0.1:1");

            assert!(fetch_configuration(&local).await.is_err());
        }
    }

    mod sync_config_from_server_tests {
        use super::*;

        async fn serve(
            server: &mut mockito::ServerGuard,
            local: &AgentConfig,
            body: &AgentConfigPatch,
        ) {
            server
                .mock(
                    "GET",
                    format!("/agent/{}/configuration", local.agent_id).as_str(),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(serde_json::to_string(body).unwrap())
                .create_async()
                .await;
        }

        #[tokio::test]
        async fn persists_merged_config_with_secure_permissions() {
            let mut server = mockito::Server::new_async().await;
            let local = local_config(&server.url());
            serve(
                &mut server,
                &local,
                &envelope(&local, 2, json!({"monitoring": {"interval_secs": 30}})),
            )
            .await;
            let file = NamedTempFile::new().unwrap();

            let version = sync_config_from_server(&local, file.path()).await.unwrap();

            assert_eq!(version, 2);
            let saved = AgentConfig::from_file(file.path()).unwrap();
            assert_eq!(saved.version, 2);
            assert_eq!(saved.agent_name, "from-server");
            assert_eq!(saved.monitoring.interval_secs, 30);
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
            serve(&mut server, &local, &envelope(&local, 1, json!({}))).await;
            let file = NamedTempFile::new().unwrap();
            std::fs::write(file.path(), "original").unwrap();

            assert!(sync_config_from_server(&local, file.path()).await.is_err());

            assert_eq!(std::fs::read_to_string(file.path()).unwrap(), "original");
        }

        #[tokio::test]
        async fn leaves_file_untouched_when_patch_has_unknown_field() {
            let mut server = mockito::Server::new_async().await;
            let local = local_config(&server.url());
            serve(
                &mut server,
                &local,
                &envelope(&local, 2, json!({"bogus": 1})),
            )
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
            serve(
                &mut server,
                &local,
                &envelope(&local, 2, json!({"server": {"verify_tls": false}})),
            )
            .await;
            let file = NamedTempFile::new().unwrap();
            std::fs::write(file.path(), "original").unwrap();

            assert!(sync_config_from_server(&local, file.path()).await.is_err());

            assert_eq!(std::fs::read_to_string(file.path()).unwrap(), "original");
        }
    }

    mod touches_path_tests {
        use super::*;

        // Tests for touches_path behavior would go here
        #[test]
        fn touches_path_success() {
            let patch = json!({"a": "b"});
            let path = &["a"];

            assert!(
                touches_path(&patch, path),
                "Patch should touch the specified path"
            );
        }

        #[test]
        fn touches_path_failure() {
            let patch = json!({"a": "b"});
            let path = &["c"];

            assert!(
                !touches_path(&patch, path),
                "Patch should not touch the specified path"
            );
        }

        #[test]
        fn touches_path_nested() {
            let patch = json!({"a": {"b": "c"}});
            let path = &["a", "b"];

            assert!(
                touches_path(&patch, path),
                "Patch should touch the specified nested path"
            );
        }
    }
}
