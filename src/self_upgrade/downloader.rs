use crate::error::{Error, Result};
use flate2::read::GzDecoder;
use semver::Version;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::Archive;
use tokio::fs;
use uuid::Uuid;

pub async fn download_release_binary(
    client: &reqwest::Client,
    check_url: &str,
    version: &Version,
) -> Result<PathBuf> {
    let base = check_url.trim_end_matches('/');
    let target = release_target();
    let version_str = version.to_string();
    let artifact_name = format!("agent-v{}-{}.tar.gz", version_str, target);
    let release_url = format!(
        "{}/releases/download/v{}/{}",
        base, version_str, artifact_name
    );
    let checksum_url = format!("{}.sha256", release_url);

    let archive_bytes = download_bytes(client, &release_url).await?;
    let checksum_text = download_text(client, &checksum_url).await?;

    verify_sha256(&archive_bytes, &checksum_text)?;

    let tmp_dir = std::env::temp_dir().join(format!("smotra-upgrade-{}", Uuid::now_v7()));
    fs::create_dir_all(&tmp_dir).await.map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("create temp dir: {}", e),
        ))
    })?;

    let archive_path = tmp_dir.join(&artifact_name);
    fs::write(&archive_path, &archive_bytes)
        .await
        .map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("write archive to a tmp file: {}", e),
            ))
        })?;

    extract_binary(&archive_path, &tmp_dir)
}

fn verify_sha256(payload: &[u8], checksum_text: &str) -> Result<()> {
    let expected = checksum_text
        .split_whitespace()
        .next()
        .ok_or_else(|| Error::Config("Checksum response is empty".to_string()))?
        .to_ascii_lowercase();

    let mut hasher = Sha256::new();
    hasher.update(payload);
    let actual = hex::encode(hasher.finalize());

    if actual != expected {
        return Err(Error::SelfUpgrade(format!(
            "Checksum mismatch: expected {}, got {}",
            expected, actual
        )));
    }

    Ok(())
}

fn extract_binary(archive_path: &Path, output_dir: &Path) -> Result<PathBuf> {
    let file = File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    let candidates = binary_candidates();

    let entries = archive
        .entries()
        .map_err(|e| Error::SelfUpgrade(format!("Failed to read archive entries: {}", e)))?;

    for entry in entries {
        let mut entry =
            entry.map_err(|e| Error::SelfUpgrade(format!("Invalid archive entry: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| Error::SelfUpgrade(format!("Invalid archive path: {}", e)))?;

        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if candidates.iter().any(|candidate| candidate == &file_name) {
                let extracted = output_dir.join(file_name);
                let mut out = File::create(&extracted)?;
                std::io::copy(&mut entry, &mut out)?;
                set_executable_permissions_if_needed(&extracted)?;
                return Ok(extracted);
            }
        }
    }

    Err(Error::SelfUpgrade(
        "Release archive does not contain expected executable (smotra/agent)".to_string(),
    ))
}

#[cfg(unix)]
fn set_executable_permissions_if_needed(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)?;
    let mut perms = metadata.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable_permissions_if_needed(_path: &Path) -> Result<()> {
    Ok(())
}

async fn download_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| Error::Network(format!("Failed to download '{}': {}", url, e)))?;

    if !response.status().is_success() {
        return Err(Error::Network(format!(
            "Download '{}' returned HTTP {}",
            url,
            response.status()
        )));
    }

    let body = response
        .bytes()
        .await
        .map_err(|e| Error::Network(format!("Failed to read body '{}': {}", url, e)))?;

    Ok(body.to_vec())
}

async fn download_text(client: &reqwest::Client, url: &str) -> Result<String> {
    let bytes = download_bytes(client, url).await?;
    String::from_utf8(bytes)
        .map_err(|e| Error::Config(format!("Expected UTF-8 text at '{}': {}", url, e)))
}

fn release_target() -> &'static str {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "aarch64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        "aarch64-pc-windows-msvc"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else {
        "x86_64-unknown-linux-gnu"
    }
}

fn binary_candidates() -> &'static [&'static str] {
    #[cfg(target_os = "windows")]
    {
        return &["smotra.exe", "agent.exe"];
    }
    #[cfg(not(target_os = "windows"))]
    {
        &["smotra", "agent"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::Builder;
    use tempfile::tempdir;

    #[test]
    /// Verifies checksum validation succeeds when digest matches.
    fn verify_sha256_accepts_matching_digest() {
        let payload = b"abc";
        let checksum = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad file";
        let result = verify_sha256(payload, checksum);
        assert!(result.is_ok(), "expected checksum validation to pass");
    }

    #[test]
    /// Verifies checksum validation fails when digest differs.
    fn verify_sha256_rejects_mismatched_digest() {
        let payload = b"abc";
        let checksum = "deadbeef file";
        let result = verify_sha256(payload, checksum);
        assert!(result.is_err(), "expected checksum validation to fail");
    }

    #[test]
    /// Verifies tar.gz extraction finds the executable file.
    fn extract_binary_finds_executable_candidate() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("artifact.tar.gz");

        let tar_gz = File::create(&archive_path).unwrap();
        let encoder = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = Builder::new(encoder);

        let bin_name = binary_candidates()[0];
        let data = b"binary";
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o755);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        tar.append_data(&mut header, bin_name, std::io::Cursor::new(data))
            .unwrap();
        let encoder = tar.into_inner().unwrap();
        let _file = encoder.finish().unwrap();

        let extracted = extract_binary(&archive_path, dir.path()).unwrap();
        assert!(extracted.exists(), "extracted binary should exist");
        assert_eq!(
            extracted.file_name().and_then(|n| n.to_str()),
            Some(bin_name),
            "extracted file name should match candidate"
        );

        let contents = std::fs::read(&extracted).unwrap();
        assert_eq!(
            contents, data,
            "extracted file contents should match original"
        );
    }
}
