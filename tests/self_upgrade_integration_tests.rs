use flate2::write::GzEncoder;
use flate2::Compression;
use mockito::Server;
use octocrab::Octocrab;
use semver::Version;
use sha2::{Digest, Sha256};
use smotra::self_upgrade::{download_release_binary, fetch_latest_version};
use std::io::Cursor;
use tar::Builder;

/// GitHub repository URL used across all tests.
/// `parse_github_url` validates it is a real github.com URL; the octocrab
/// client is configured with `base_uri` to redirect actual HTTP calls to the
/// local mockito server.
const TEST_REPO_URL: &str = "https://github.com/test-owner/test-repo";

/// Builds an octocrab client whose GitHub API base URI is redirected to the
/// given mock server URL. Asset download URLs embedded in mock responses also
/// point at the mock server, so `octocrab.download()` hits the mock too.
fn build_octocrab_for_mock(server_url: &str) -> Octocrab {
    Octocrab::builder()
        .base_uri(server_url)
        .expect("valid mock server URL")
        .build()
        .expect("build octocrab client")
}

/// Builds the minimal GitHub API release JSON that octocrab will deserialise.
/// `browser_download_url` for each asset points at the mock server so that
/// `octocrab.download()` calls are intercepted without leaving the test process.
fn release_json(server_url: &str, version: &str) -> String {
    let base = server_url.trim_end_matches('/');
    let target = test_release_target();
    let artifact_name = format!("smotra-v{}-{}.tar.gz", version, target);
    let checksum_name = format!("{}.sha256", artifact_name);
    let user = minimal_user_json();

    serde_json::json!({
        "url": format!("{}/repos/test-owner/test-repo/releases/1", base),
        "assets_url": format!("{}/repos/test-owner/test-repo/releases/1/assets", base),
        "upload_url": format!("{}/repos/test-owner/test-repo/releases/1/assets{{?name,label}}", base),
        "html_url": format!("https://github.com/test-owner/test-repo/releases/tag/v{}", version),
        "id": 1,
        "node_id": "RE_kwDOTest123",
        "tag_name": format!("v{}", version),
        "target_commitish": "main",
        "name": format!("v{}", version),
        "draft": false,
        "prerelease": false,
        "created_at": "2024-01-01T00:00:00Z",
        "published_at": "2024-01-01T00:00:00Z",
        "author": user,
        "assets": [
            asset_json(base, version, &artifact_name, 1),
            asset_json(base, version, &checksum_name, 2),
        ],
        "tarball_url": null,
        "zipball_url": null,
        "body": "Test release"
    })
    .to_string()
}

fn asset_json(base: &str, version: &str, name: &str, id: u64) -> serde_json::Value {
    serde_json::json!({
        "url": format!("{}/repos/test-owner/test-repo/releases/assets/{}", base, id),
        "id": id,
        "node_id": format!("RA_kwDOTest{}", id),
        "name": name,
        "label": null,
        "uploader": minimal_user_json(),
        "content_type": "application/octet-stream",
        "state": "uploaded",
        "size": 100,
        "download_count": 0,
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z",
        "browser_download_url": format!("{}/releases/download/v{}/{}", base, version, name)
    })
}

fn minimal_user_json() -> serde_json::Value {
    serde_json::json!({
        "login": "test-user",
        "id": 1,
        "node_id": "MDQ6VXNlcjE=",
        "avatar_url": "https://avatars.githubusercontent.com/u/1?v=4",
        "gravatar_id": "",
        "url": "https://api.github.com/users/test-user",
        "html_url": "https://github.com/test-user",
        "followers_url": "https://api.github.com/users/test-user/followers",
        "following_url": "https://api.github.com/users/test-user/following{/other_user}",
        "gists_url": "https://api.github.com/users/test-user/gists{/gist_id}",
        "starred_url": "https://api.github.com/users/test-user/starred{/owner}{/repo}",
        "subscriptions_url": "https://api.github.com/users/test-user/subscriptions",
        "organizations_url": "https://api.github.com/users/test-user/orgs",
        "repos_url": "https://api.github.com/users/test-user/repos",
        "events_url": "https://api.github.com/users/test-user/events{/privacy}",
        "received_events_url": "https://api.github.com/users/test-user/received_events",
        "type": "User",
        "site_admin": false
    })
}

#[tokio::test]
/// Verifies that fetch_latest_version parses the tag_name from the GitHub
/// Releases API response, stripping a leading "v" prefix.
async fn fetch_latest_version_parses_v_prefixed_version() {
    let mut server = Server::new_async().await;
    let body = release_json(&server.url(), "1.2.3");

    let _mock = server
        .mock("GET", "/repos/test-owner/test-repo/releases/latest")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let octocrab = build_octocrab_for_mock(&server.url());
    let latest = fetch_latest_version(&octocrab, TEST_REPO_URL)
        .await
        .expect("fetch latest version should succeed");

    assert_eq!(latest, Version::parse("1.2.3").unwrap());
}

#[tokio::test]
/// Verifies that download_release_binary fetches the release by tag, finds
/// the matching asset and checksum asset, verifies the SHA-256 digest, and
/// returns a path to the extracted binary with the correct contents.
async fn download_release_binary_downloads_verifies_and_extracts() {
    let mut server = Server::new_async().await;
    let version = Version::parse("1.2.3").unwrap();
    let target = test_release_target();
    let artifact_name = format!("smotra-v{}-{}.tar.gz", version, target);
    let checksum_name = format!("{}.sha256", artifact_name);

    let archive = build_archive_with_binary(test_binary_name(), b"new-binary-contents");
    let mut hasher = Sha256::new();
    hasher.update(&archive);
    let digest = hex::encode(hasher.finalize());

    let release_body = release_json(&server.url(), &version.to_string());
    let _release_mock = server
        .mock(
            "GET",
            format!("/repos/test-owner/test-repo/releases/tags/v{}", version).as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(release_body)
        .create_async()
        .await;

    let artifact_path = format!("/releases/download/v{}/{}", version, artifact_name);
    let checksum_path = format!("/releases/download/v{}/{}", version, checksum_name);

    let _artifact_mock = server
        .mock("GET", artifact_path.as_str())
        .with_status(200)
        .with_body(archive)
        .create_async()
        .await;

    let _checksum_mock = server
        .mock("GET", checksum_path.as_str())
        .with_status(200)
        .with_body(format!("{}  {}", digest, artifact_name))
        .create_async()
        .await;

    let octocrab = build_octocrab_for_mock(&server.url());
    let extracted = download_release_binary(&octocrab, TEST_REPO_URL, &version)
        .await
        .expect("download and extract should succeed");

    assert!(extracted.exists(), "extracted binary should exist");
    let payload = std::fs::read(&extracted).expect("should read extracted binary");
    assert_eq!(payload, b"new-binary-contents");
}

fn build_archive_with_binary(name: &str, body: &[u8]) -> Vec<u8> {
    let mut compressed_payload = Vec::new();
    {
        let encoder = GzEncoder::new(&mut compressed_payload, Compression::default());
        let mut tar = Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(0o755);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        tar.append_data(&mut header, name, Cursor::new(body))
            .expect("append binary to archive");
        let encoder = tar.into_inner().expect("flush tar archive into gzip");
        let _sink = encoder.finish().expect("finalize gzip stream");
    }

    compressed_payload
}

fn test_binary_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        return "smotra.exe";
    }

    #[cfg(not(target_os = "windows"))]
    {
        "smotra"
    }
}

fn test_release_target() -> &'static str {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "aarch64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        "aarch64-pc-windows-msvc"
    } else {
        "x86_64-unknown-linux-gnu"
    }
}
