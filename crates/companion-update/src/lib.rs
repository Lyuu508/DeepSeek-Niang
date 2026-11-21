//! Only the upstream repository's stable releases are install candidates.
use reqwest::{Client, Response, Url, redirect};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path, time::Duration};
use tokio::io::AsyncWriteExt;

const LATEST: &str = "https://api.github.com/repos/Lyuu508/DeepSeek-Niang/releases/latest";
const RELEASES: &str = "https://github.com/Lyuu508/DeepSeek-Niang/releases/download/";
const MAX_PACKAGE: u64 = 384 * 1024 * 1024;
const SUMS: &str = "SHA256SUMS.txt";
type Result<T> = std::result::Result<T, &'static str>;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    published_at: String,
    body: Option<String>,
    assets: Vec<Asset>,
}

#[derive(Deserialize, Clone)]
struct Asset {
    name: String,
    browser_download_url: String,
    state: String,
    size: u64,
    digest: Option<String>,
}

#[derive(Clone, Copy)]
pub enum Platform {
    WindowsX64,
    AndroidArm64,
    Unavailable,
}

impl Platform {
    fn filename(self, version: &Version) -> Option<String> {
        let suffix = match self {
            Self::WindowsX64 => "windows-x64-setup.exe",
            Self::AndroidArm64 => "android-arm64.apk",
            Self::Unavailable => return None,
        };
        Some(format!("DeepSeek-Niang_{version}_{suffix}"))
    }
}

#[derive(Clone, Serialize)]
pub struct UpdateView {
    pub version: String,
    pub published_at: String,
    pub notes: String,
    pub newer: bool,
    pub can_download: bool,
}

#[derive(Clone)]
pub struct Latest {
    pub view: UpdateView,
    package: Option<Asset>,
    sums: Option<Asset>,
}

pub struct Download {
    _directory: tempfile::TempDir,
    pub path: std::path::PathBuf,
    pub sha256: String,
    pub version: String,
}

fn parsed_release(release: Release, current: &str, platform: Platform) -> Result<Latest> {
    let version = release
        .tag_name
        .strip_prefix('v')
        .and_then(|tag| Version::parse(tag).ok())
        .filter(|version| version.pre.is_empty() && version.build.is_empty())
        .ok_or("UPDATE_METADATA_INVALID")?;
    if release.draft || release.prerelease || release.published_at.is_empty() {
        return Err("UPDATE_METADATA_INVALID");
    }
    let current = Version::parse(current).map_err(|_| "UPDATE_METADATA_INVALID")?;
    let newer = version > current;
    let find = |name: &str, max| {
        let mut matches = release.assets.iter().filter(|asset| asset.name == name);
        let asset = matches.next()?;
        let expected = format!("{RELEASES}{}/{name}", release.tag_name);
        (matches.next().is_none()
            && asset.browser_download_url == expected
            && asset.state == "uploaded"
            && (1..=max).contains(&asset.size))
        .then(|| asset.clone())
    };
    let package = platform
        .filename(&version)
        .and_then(|name| find(&name, MAX_PACKAGE));
    let sums = find(SUMS, 65_536);
    let view = UpdateView {
        version: version.to_string(),
        published_at: release.published_at,
        notes: release
            .body
            .unwrap_or_default()
            .chars()
            .take(16_000)
            .collect(),
        newer,
        can_download: newer && package.is_some() && sums.is_some(),
    };
    Ok(Latest {
        view,
        package,
        sums,
    })
}

fn redirect_is_safe(url: &Url) -> bool {
    url.scheme() == "https"
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
        && match url.host_str() {
            Some("release-assets.githubusercontent.com" | "objects.githubusercontent.com") => true,
            Some("github.com") => url.as_str().starts_with(RELEASES),
            _ => false,
        }
}

pub struct Updater {
    client: Client,
}

impl Updater {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .https_only(true)
            .redirect(redirect::Policy::custom(|attempt| {
                if attempt.previous().len() < 4
                    && attempt
                        .previous()
                        .first()
                        .is_some_and(|from| from.as_str().starts_with(RELEASES))
                    && redirect_is_safe(attempt.url())
                {
                    attempt.follow()
                } else {
                    attempt.error("unexpected release redirect")
                }
            }))
            .connect_timeout(Duration::from_secs(12))
            .read_timeout(Duration::from_secs(35))
            .timeout(Duration::from_secs(420))
            .user_agent("DeepSeekCompanion/0.1 updater")
            .build()
            .map_err(network)?;
        Ok(Self { client })
    }

    pub async fn check(&self, current: &str, platform: Platform) -> Result<Option<Latest>> {
        let response = self
            .client
            .get(LATEST)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .timeout(Duration::from_secs(20))
            .send()
            .await
            .map_err(network)?;
        if response.status() == 404 {
            return Ok(None);
        }
        let release: Release = serde_json::from_slice(&bytes(response, 256 * 1024).await?)
            .map_err(|_| "UPDATE_METADATA_INVALID")?;
        let is_acceptance_tag = release
            .tag_name
            .strip_prefix('v')
            .and_then(|tag| Version::parse(tag).ok())
            .is_some_and(|version| !version.pre.is_empty());
        if release.prerelease || release.draft || is_acceptance_tag {
            return Ok(None);
        }
        Ok(Some(parsed_release(release, current, platform)?))
    }

    pub async fn download(
        &self,
        latest: &Latest,
        directory: &Path,
        mut progress: impl FnMut(u8),
    ) -> Result<Download> {
        if !latest.view.can_download {
            return Err("UPDATE_ASSET_MISSING");
        }
        let sums = latest.sums.as_ref().ok_or("UPDATE_ASSET_MISSING")?;
        let package = latest.package.as_ref().ok_or("UPDATE_ASSET_MISSING")?;
        let response = self
            .client
            .get(&sums.browser_download_url)
            .send()
            .await
            .map_err(network)?;
        let checksum = bytes(response, 65_536).await?;
        if checksum.len() as u64 != sums.size {
            return Err("UPDATE_CHECKSUM_INVALID");
        }
        check_github_digest(sums, &hex(Sha256::digest(&checksum)))?;
        let sha256 = checksum_for(&checksum, &package.name)?;
        check_github_digest(package, &sha256)?;
        std::fs::create_dir_all(directory).map_err(storage)?;
        let directory = tempfile::Builder::new()
            .prefix("release-")
            .tempdir_in(directory)
            .map_err(storage)?;
        let path = directory.path().join(&package.name);
        let mut file = tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .await
            .map_err(storage)?;
        let response = self
            .client
            .get(&package.browser_download_url)
            .send()
            .await
            .map_err(network)?;
        let mut response = status(response)?;
        let mut digest = Sha256::new();
        let mut count = 0_u64;
        let mut percent = 0_u8;
        progress(percent);
        while let Some(chunk) = response.chunk().await.map_err(network)? {
            count += chunk.len() as u64;
            if count > package.size {
                return Err("UPDATE_CHECKSUM_INVALID");
            }
            digest.update(&chunk);
            file.write_all(&chunk).await.map_err(storage)?;
            let received = (count.saturating_mul(100) / package.size) as u8;
            if received > percent {
                percent = received;
                progress(percent);
            }
        }
        file.sync_all().await.map_err(storage)?;
        drop(file);
        if count != package.size || hex(digest.finalize()) != sha256 {
            return Err("UPDATE_CHECKSUM_INVALID");
        }
        Ok(Download {
            _directory: directory,
            path,
            sha256,
            version: latest.view.version.clone(),
        })
    }
}

impl Download {
    /// The OS installer still needs the staged file after this application exits.
    pub fn retain(self) {
        let _ = self._directory.keep();
    }
}

fn storage<T>(_: T) -> &'static str {
    "UPDATE_STORAGE_FAILED"
}
fn network(error: reqwest::Error) -> &'static str {
    if error.is_timeout() {
        "UPDATE_TIMEOUT"
    } else {
        "UPDATE_NETWORK_FAILED"
    }
}

fn status(response: Response) -> Result<Response> {
    if !response.status().is_success() {
        return Err("UPDATE_NETWORK_FAILED");
    }
    Ok(response)
}

async fn bytes(response: Response, limit: usize) -> Result<Vec<u8>> {
    let mut response = status(response)?;
    let mut data = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(network)? {
        if data.len() + chunk.len() > limit {
            return Err("UPDATE_METADATA_INVALID");
        }
        data.extend_from_slice(&chunk);
    }
    Ok(data)
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn check_github_digest(asset: &Asset, sha256: &str) -> Result<()> {
    if asset
        .digest
        .as_ref()
        .is_some_and(|digest| digest != &format!("sha256:{sha256}"))
    {
        return Err("UPDATE_CHECKSUM_INVALID");
    }
    Ok(())
}

fn checksum_for(bytes: &[u8], filename: &str) -> Result<String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "UPDATE_CHECKSUM_INVALID")?;
    let mut matching = text.lines().filter_map(|line| {
        let (digest, name) = line.trim().split_once(char::is_whitespace)?;
        (name.trim_start().trim_start_matches('*') == filename).then_some(digest)
    });
    let digest = matching.next().ok_or("UPDATE_CHECKSUM_INVALID")?;
    if matching.next().is_some()
        || digest.len() != 64
        || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("UPDATE_CHECKSUM_INVALID");
    }
    Ok(digest.to_ascii_lowercase())
}

pub fn verify_file(path: &Path, expected: &str) -> Result<()> {
    let mut reader = std::fs::File::open(path)
        .map_err(storage)?
        .take(MAX_PACKAGE + 1);
    let mut digest = Sha256::new();
    let mut count = 0_u64;
    let mut buffer = [0_u8; 65_536];
    loop {
        let length = reader.read(&mut buffer).map_err(storage)?;
        if length == 0 {
            break;
        }
        count += length as u64;
        digest.update(&buffer[..length]);
    }
    if count == 0 || count > MAX_PACKAGE || hex(digest.finalize()) != expected {
        return Err("UPDATE_CHECKSUM_INVALID");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn release(tag: &str) -> Release {
        let assets = ["DeepSeek-Niang_0.1.1_windows-x64-setup.exe", SUMS].map(|name|
            json!({"name":name,"browser_download_url":format!("{RELEASES}{tag}/{name}"),"state":"uploaded","size":123}));
        serde_json::from_value(json!({"tag_name":tag,"draft":false,"prerelease":false,
            "published_at":"2026-11-21T00:00:00Z","body":"changes","assets":assets}))
        .unwrap()
    }

    #[test]
    fn stable_exact_assets_are_required_and_versions_are_numeric() {
        let latest = parsed_release(release("v0.1.1"), "0.0.10", Platform::WindowsX64).unwrap();
        assert!(latest.view.newer && latest.view.can_download);
        assert!(
            !parsed_release(release("v0.1.1"), "0.1.10", Platform::WindowsX64)
                .unwrap()
                .view
                .newer
        );
        assert!(
            !parsed_release(release("v0.1.1"), "0.1.0", Platform::AndroidArm64)
                .unwrap()
                .view
                .can_download
        );
        let mut draft = release("v0.1.1");
        draft.draft = true;
        assert!(parsed_release(draft, "0.1.0", Platform::WindowsX64).is_err());
        let mut prerelease = release("v0.1.1");
        prerelease.prerelease = true;
        assert!(parsed_release(prerelease, "0.1.0", Platform::WindowsX64).is_err());
        let mut foreign = release("v0.1.1");
        foreign.assets[0].browser_download_url = "https://elsewhere.example/file.exe".into();
        assert!(
            !parsed_release(foreign, "0.1.0", Platform::WindowsX64)
                .unwrap()
                .view
                .can_download
        );
    }

    #[test]
    fn exact_checksum_rejects_ambiguous_and_tampered_files() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("fixture.exe");
        std::fs::write(&file, b"synthetic package").unwrap();
        let sha = hex(Sha256::digest(b"synthetic package"));
        let line = format!("{sha}  fixture.exe\n");
        assert_eq!(checksum_for(line.as_bytes(), "fixture.exe").unwrap(), sha);
        assert!(checksum_for(line.repeat(2).as_bytes(), "fixture.exe").is_err());
        assert!(checksum_for(line.as_bytes(), "other.exe").is_err());
        assert!(verify_file(&file, &sha).is_ok());
        std::fs::write(&file, b"different bytes").unwrap();
        assert!(verify_file(&file, &sha).is_err());
        assert!(redirect_is_safe(
            &Url::parse("https://release-assets.githubusercontent.com/package?temporary=1")
                .unwrap()
        ));
        assert!(!redirect_is_safe(
            &Url::parse("http://release-assets.githubusercontent.com/package").unwrap()
        ));
        assert!(!redirect_is_safe(
            &Url::parse("https://release-assets.githubusercontent.com.example.test/package")
                .unwrap()
        ));
    }
}
