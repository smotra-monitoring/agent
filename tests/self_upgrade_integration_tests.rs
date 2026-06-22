use flate2::write::GzEncoder;
use flate2::Compression;
use mockito::Server;
use semver::Version;
use sha2::{Digest, Sha256};
use smotra::self_updater::{download_release_binary, fetch_latest_version};
use std::io::Cursor;
use tar::Builder;

#[tokio::test]
async fn fetch_latest_version_parses_v_prefixed_version() {
    let mut server = Server::new_async().await;

    let _mock = server
        .mock("GET", "/releases/latest/version.txt")
        .with_status(200)
        .with_body("v1.2.3\n")
        .create_async()
        .await;

    let client = reqwest::Client::new();
    let latest = fetch_latest_version(&client, &server.url())
        .await
        .expect("fetch latest version should succeed");

    assert_eq!(latest, Version::parse("1.2.3").unwrap());
}

#[tokio::test]
async fn download_release_binary_downloads_verifies_and_extracts() {
    let mut server = Server::new_async().await;
    let version = Version::parse("1.2.3").unwrap();
    let target = test_release_target();
    let artifact_name = format!("agent-v{}-{}.tar.gz", version, target);

    let archive = build_archive_with_binary(test_binary_name(), b"new-binary-contents");
    let mut hasher = Sha256::new();
    hasher.update(&archive);
    let digest = hex::encode(hasher.finalize());

    let artifact_path = format!("/releases/download/v{}/{}", version, artifact_name);
    let checksum_path = format!("{}.sha256", artifact_path);

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

    let client = reqwest::Client::new();
    let extracted = download_release_binary(&client, &server.url(), &version)
        .await
        .expect("download and extract should succeed");

    assert!(extracted.exists(), "extracted binary should exist");
    let payload = std::fs::read(&extracted).expect("should read extracted binary");
    assert_eq!(payload, b"new-binary-contents");
}

fn build_archive_with_binary(name: &str, body: &[u8]) -> Vec<u8> {
    let mut tar_payload = Vec::new();
    {
        let encoder = GzEncoder::new(&mut tar_payload, Compression::default());
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

    tar_payload
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
