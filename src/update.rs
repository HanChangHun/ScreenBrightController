//! Read-only release validation and byte integrity; no network, process or gamma APIs.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub const MAX_DOWNLOAD: u64 = 64 * 1024 * 1024;
pub const LATEST: &str =
    "https://api.github.com/repos/HanChangHun/ScreenBrightController/releases/latest";
#[derive(Clone, Serialize)]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub name: String,
    pub size: u64,
    pub installer_url: String,
    pub checksum_url: String,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
    browser_download_url: String,
}
#[derive(Deserialize)]
struct Manifest {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    body: Option<String>,
    assets: Vec<Asset>,
}
fn version(text: &str) -> Result<semver::Version, String> {
    let v = semver::Version::parse(text).map_err(|_| "Invalid release version")?;
    if !v.pre.is_empty() || !v.build.is_empty() || v.to_string() != text {
        return Err("Only canonical stable versions are accepted".into());
    }
    Ok(v)
}
impl Release {
    pub fn parse(json: &str, current: &str) -> Result<Option<Self>, String> {
        if json.len() > 1024 * 1024 {
            return Err("Release metadata too large".into());
        }
        let m: Manifest = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if m.draft || m.prerelease {
            return Err("Only published stable releases are accepted".into());
        }
        let text = m
            .tag_name
            .strip_prefix('v')
            .ok_or("Release tag must start with v")?;
        let next = version(text)?;
        if next <= version(current)? {
            return Ok(None);
        }
        let name = format!("ScreenBrightController_{text}_x64-setup.exe");
        let base = format!(
            "https://github.com/HanChangHun/ScreenBrightController/releases/download/{}",
            m.tag_name
        );
        let asset = |name: &str, max: u64| -> Result<&Asset, String> {
            let mut matches = m.assets.iter().filter(|a| a.name == name);
            let a = matches.next().ok_or("Release asset missing")?;
            if matches.next().is_some()
                || a.size == 0
                || a.size > max
                || a.browser_download_url != format!("{base}/{name}")
            {
                return Err("Invalid or duplicate release asset".into());
            }
            Ok(a)
        };
        let installer = asset(&name, MAX_DOWNLOAD)?;
        let sums = asset("SHA256SUMS.txt", 8192)?;
        Ok(Some(Self {
            version: text.into(),
            notes: m.body.clone().unwrap_or_default(),
            name,
            size: installer.size,
            installer_url: installer.browser_download_url.clone(),
            checksum_url: sums.browser_download_url.clone(),
        }))
    }
}
pub fn expected_checksum(sums: &str, name: &str) -> Result<String, String> {
    if sums.len() > 8192 {
        return Err("Checksum list too large".into());
    }
    let mut found = None;
    for line in sums.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() == 2 && fields[1] == name {
            if found.is_some()
                || fields[0].len() != 64
                || !fields[0].bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err("Invalid or duplicate installer checksum".into());
            }
            found = Some(fields[0].to_ascii_lowercase());
        }
    }
    found.ok_or("Installer checksum missing".into())
}
pub fn verify_bytes(bytes: &[u8], size: u64, expected: &str) -> Result<(), String> {
    if bytes.len() as u64 != size || size == 0 || size > MAX_DOWNLOAD {
        return Err("Installer size mismatch".into());
    }
    if format!("{:x}", Sha256::digest(bytes)) != expected {
        return Err("Installer SHA-256 mismatch".into());
    }
    Ok(())
}
