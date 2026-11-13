use crate::{
    common::{display_version_from_release_id, release_id_from_update_url},
    hbbs_http::create_http_client_with_url_strict,
    update_metadata::{UpdateArtifactQuery, UpdateMetadataRequirements, VerifiedUpdateArtifact},
};
use hbb_common::{bail, ResultType};
use serde_derive::Deserialize;
use std::{
    io::Read,
    path::{Component, Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

const UPDATE_SIDECAR_HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const UPDATE_METADATA_SIDECAR_MAX_BYTES: u64 = 1024 * 1024;

lazy_static::lazy_static! {
    static ref VERIFIED_UPDATE_ARTIFACT: Mutex<Option<VerifiedUpdateArtifact>> = Mutex::new(None);
}

pub(crate) fn current_update_platform() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::env::consts::OS
    }
}

pub(crate) fn current_update_arch() -> &'static str {
    #[cfg(all(target_os = "windows", not(feature = "flutter")))]
    {
        "x86"
    }
    #[cfg(not(all(target_os = "windows", not(feature = "flutter"))))]
    {
        std::env::consts::ARCH
    }
}

pub(crate) fn current_update_format(update_msi: bool) -> &'static str {
    #[cfg(any(
        not(target_os = "windows"),
        all(target_os = "windows", not(feature = "flutter"))
    ))]
    let _ = update_msi;
    #[cfg(all(target_os = "windows", feature = "flutter"))]
    {
        if update_msi {
            return "msi";
        }
        "exe"
    }
    #[cfg(all(target_os = "windows", not(feature = "flutter")))]
    {
        "exe"
    }
    #[cfg(target_os = "macos")]
    {
        "dmg"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        "unknown"
    }
}

pub fn current_update_artifact_query(update_msi: bool) -> UpdateArtifactQuery<'static> {
    UpdateArtifactQuery {
        platform: current_update_platform(),
        arch: current_update_arch(),
        format: current_update_format(update_msi),
    }
}

pub fn verified_update_artifact_for_release_page_url(
    release_page_url: &str,
    query: UpdateArtifactQuery<'_>,
) -> ResultType<VerifiedUpdateArtifact> {
    let artifact = verified_update_artifact_from_release_page_url(release_page_url, &query)?;
    *VERIFIED_UPDATE_ARTIFACT.lock().unwrap() = Some(artifact.clone());
    Ok(artifact)
}

pub fn verified_update_artifact_for_download_url(
    download_url: &str,
) -> ResultType<VerifiedUpdateArtifact> {
    VERIFIED_UPDATE_ARTIFACT
        .lock()
        .unwrap()
        .as_ref()
        .filter(|artifact| artifact.url == download_url)
        .cloned()
        .ok_or_else(|| hbb_common::anyhow::anyhow!("update artifact was not verified"))
}

pub(super) fn verified_update_artifact_from_release_page_url(
    update_url: &str,
    query: &UpdateArtifactQuery<'_>,
) -> ResultType<VerifiedUpdateArtifact> {
    if crate::branding::locks_server() && crate::branding::is_fork_update_page(update_url) {
        return verified_fork_update_artifact(query);
    }
    let release_id = release_id_from_update_url(update_url)?;
    let display_version = display_version_from_release_id(&release_id)?;
    let expected_artifact_url_prefix =
        format!("https://github.com/rustdesk/rustdesk/releases/download/{release_id}/");
    let metadata_url = format!("{expected_artifact_url_prefix}rustdesk-update.json");
    let signature_url = format!("{metadata_url}.sig");
    let metadata_bytes = fetch_update_sidecar_bytes(&metadata_url)?;
    let signature_bytes = fetch_update_sidecar_bytes(&signature_url)?;
    let requirements = UpdateMetadataRequirements {
        expected_version: display_version.as_str(),
        expected_release_id: release_id.as_str(),
        expected_artifact_url_prefix: expected_artifact_url_prefix.as_str(),
        artifact: *query,
    };
    crate::update_metadata::verify_update_metadata(&metadata_bytes, &signature_bytes, requirements)
}

fn fetch_update_sidecar_bytes(url: &str) -> ResultType<Vec<u8>> {
    let client = create_http_client_with_url_strict(url)?;
    let mut response = client
        .get(url)
        .timeout(UPDATE_SIDECAR_HTTP_REQUEST_TIMEOUT)
        .send()?;
    if !response.status().is_success() {
        bail!(
            "Failed to download update metadata sidecar: {}",
            response.status()
        );
    }
    read_limited_response_bytes(
        &mut response,
        UPDATE_METADATA_SIDECAR_MAX_BYTES,
        "Update metadata sidecar",
    )
}

fn read_limited_response_bytes<R: Read>(
    reader: &mut R,
    limit: u64,
    what: &str,
) -> ResultType<Vec<u8>> {
    let mut limited_reader = reader.take(limit.saturating_add(1));
    let mut bytes = Vec::new();
    limited_reader.read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("{what} exceeds maximum allowed size of {limit} bytes");
    }
    Ok(bytes)
}

#[derive(Deserialize)]
struct ForkRelease {
    assets: Vec<ForkAsset>,
}

#[derive(Deserialize)]
struct ForkAsset {
    name: String,
    size: u64,
    digest: Option<String>,
    browser_download_url: String,
}

pub fn fork_update_page_if_newer() -> ResultType<Option<String>> {
    let release = fetch_fork_release()?;
    let arch = current_update_arch();
    let current = hbb_common::get_version_number(crate::VERSION);
    let newer = release.assets.iter().any(|asset| {
        ["exe", "msi", "dmg"].iter().any(|format| {
            fork_asset_matches(&asset.name, arch, format)
                .is_some_and(|version| hbb_common::get_version_number(version) > current)
        })
    });
    if newer {
        Ok(Some(crate::branding::update_release_page_url()))
    } else {
        Ok(None)
    }
}

fn verified_fork_update_artifact(
    query: &UpdateArtifactQuery<'_>,
) -> ResultType<VerifiedUpdateArtifact> {
    let release = fetch_fork_release()?;
    let Some(asset) = release
        .assets
        .iter()
        .filter(|asset| fork_asset_matches(&asset.name, query.arch, query.format).is_some())
        .max_by(|left, right| {
            let left_version = fork_asset_matches(&left.name, query.arch, query.format)
                .map(hbb_common::get_version_number);
            let right_version = fork_asset_matches(&right.name, query.arch, query.format)
                .map(hbb_common::get_version_number);
            left_version.cmp(&right_version)
        })
    else {
        bail!(
            "No {} {} asset in the fork release",
            query.arch,
            query.format
        );
    };
    fork_artifact_from_asset(asset, query.arch, query.format, crate::VERSION)
}

fn fetch_fork_release() -> ResultType<ForkRelease> {
    let url = crate::branding::fork_release_api_url();
    let client = create_http_client_with_url_strict(&url)?;
    let mut response = client
        .get(&url)
        .timeout(UPDATE_SIDECAR_HTTP_REQUEST_TIMEOUT)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "arturkmat-rustdesk")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()?;
    if !response.status().is_success() {
        bail!("Failed to read fork release: {}", response.status());
    }
    let bytes = read_limited_response_bytes(
        &mut response,
        UPDATE_METADATA_SIDECAR_MAX_BYTES,
        "Fork release",
    )?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn fork_artifact_from_asset(
    asset: &ForkAsset,
    arch: &str,
    format: &str,
    current_version: &str,
) -> ResultType<VerifiedUpdateArtifact> {
    let Some(version) = fork_asset_matches(&asset.name, arch, format) else {
        bail!("Update asset {} does not match {arch}.{format}", asset.name);
    };
    if hbb_common::get_version_number(version) <= hbb_common::get_version_number(current_version) {
        bail!("Fork release {version} is not newer than {current_version}");
    }
    let expected_url = crate::branding::fork_download_url(&asset.name);
    if asset.browser_download_url != expected_url {
        bail!("Fork asset URL is not the pinned release download");
    }
    if asset.size == 0 {
        bail!("Fork asset {} has no size", asset.name);
    }
    let Some(digest) = asset.digest.as_deref() else {
        bail!("Fork asset {} has no digest", asset.name);
    };
    Ok(VerifiedUpdateArtifact {
        version: version.to_owned(),
        url: expected_url,
        #[cfg(target_os = "macos")]
        file_name: asset.name.clone(),
        size: asset.size,
        sha256: sha256_from_digest(digest)?,
    })
}

fn sha256_from_digest(digest: &str) -> ResultType<String> {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        bail!("Fork asset digest is not sha256");
    };
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("Fork asset digest is not a sha256 hex string");
    }
    Ok(hex.to_ascii_lowercase())
}

fn fork_asset_matches<'a>(filename: &'a str, arch: &str, format: &str) -> Option<&'a str> {
    let version = fork_asset_version(filename)?;
    let suffix = filename.strip_prefix("rustdesk-")?.split_once('-')?.1;
    if suffix == format!("{arch}.{format}") {
        Some(version)
    } else {
        None
    }
}

fn fork_asset_version(filename: &str) -> Option<&str> {
    let rest = filename.strip_prefix("rustdesk-")?;
    let (version, suffix) = rest.split_once('-')?;
    if !is_stable_version(version) || !is_safe_asset_suffix(suffix) {
        return None;
    }
    if suffix.ends_with(".exe")
        || suffix.ends_with(".msi")
        || suffix.ends_with(".dmg")
        || suffix.ends_with(".apk")
    {
        Some(version)
    } else {
        None
    }
}

fn is_stable_version(version: &str) -> bool {
    let mut parts = version.split('.');
    let three_numbers = (0..3).all(|_| {
        matches!(
            parts.next(),
            Some(part) if !part.is_empty() && part.chars().all(|c| c.is_ascii_digit())
        )
    });
    three_numbers && parts.next().is_none()
}

fn is_safe_asset_suffix(suffix: &str) -> bool {
    !suffix.contains("..")
        && suffix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

pub fn get_update_download_file_from_url(url: &str) -> Option<PathBuf> {
    let parsed = url::Url::parse(url).ok()?;
    // Check the raw prefix before Url normalizes default ports.
    if !url.starts_with("https://github.com/")
        || parsed.scheme() != "https"
        || parsed.host_str() != Some("github.com")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return None;
    }

    let mut segments = parsed.path_segments()?;
    let owner = segments.next()?;
    let repo = segments.next()?;
    let releases = segments.next()?;
    let download = segments.next()?;
    let tag = segments.next()?;
    let filename = segments.next()?;

    if releases != "releases"
        || download != "download"
        || segments.next().is_some()
        || !is_allowed_update_asset(owner, repo, tag, filename)
    {
        return None;
    }

    Some(std::env::temp_dir().join(filename))
}

fn is_allowed_update_asset(owner: &str, repo: &str, tag: &str, filename: &str) -> bool {
    if tag.is_empty() || !is_plain_update_filename(filename) {
        return false;
    }
    if owner == "rustdesk" && repo == "rustdesk" {
        return true;
    }
    owner == crate::branding::UPDATE_OWNER
        && repo == crate::branding::UPDATE_REPO
        && tag == crate::branding::UPDATE_TAG
        && fork_asset_version(filename).is_some()
}

fn is_plain_update_filename(filename: &str) -> bool {
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains(':')
    {
        return false;
    }

    let mut components = Path::new(filename).components();
    matches!(
        components.next(),
        Some(Component::Normal(name)) if name.to_str() == Some(filename)
    ) && components.next().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_download_file_accepts_expected_github_asset_urls() {
        let file = get_update_download_file_from_url(
            "https://github.com/rustdesk/rustdesk/releases/download/1.4.0/rustdesk-1.4.0-x86_64.dmg",
        )
        .expect("valid GitHub release asset URL");

        assert_eq!(
            file.file_name().and_then(|name| name.to_str()),
            Some("rustdesk-1.4.0-x86_64.dmg")
        );
    }

    #[test]
    fn update_download_file_rejects_untrusted_or_malformed_urls() {
        for url in [
            "http://github.com/rustdesk/rustdesk/releases/download/1/rustdesk.exe",
            "https://example.com/rustdesk.exe",
            "https://github.com/other/project/releases/download/1/rustdesk.exe",
            "https://github.com/rustdesk/rustdesk/releases/download/1/",
            "https://github.com/rustdesk/rustdesk/releases/download/1/nested/rustdesk.exe",
            "https://github.com/rustdesk/rustdesk/releases/download/1/C:rustdesk.exe",
            "https://user@github.com/rustdesk/rustdesk/releases/download/1/rustdesk.exe",
            "https://github.com:443/rustdesk/rustdesk/releases/download/1/rustdesk.exe",
            "https://github.com/rustdesk/rustdesk/releases/download/1/rustdesk.exe?download=1",
            "https://github.com/rustdesk/rustdesk/releases/download/1/rustdesk.exe#download",
            "not a url",
            "https://github.com/arturkmat/rustdesk/releases/download/evil/rustdesk-9.9.9-x86_64.exe",
            "https://github.com/arturkmat/other/releases/download/dark-smart/rustdesk-9.9.9-x86_64.exe",
        ] {
            assert!(get_update_download_file_from_url(url).is_none(), "{url}");
        }
    }

    #[test]
    fn update_download_file_accepts_pinned_fork_asset() {
        let file = get_update_download_file_from_url(
            "https://github.com/arturkmat/rustdesk/releases/download/dark-smart/rustdesk-9.9.9-x86_64.exe",
        )
        .expect("pinned fork asset");
        assert_eq!(
            file.file_name().and_then(|name| name.to_str()),
            Some("rustdesk-9.9.9-x86_64.exe")
        );
    }

    #[test]
    fn fork_asset_uses_github_digest_and_rejects_a_swapped_url() {
        let name = "rustdesk-9.9.9-x86_64.exe";
        let good = ForkAsset {
            name: name.to_owned(),
            size: 42,
            digest: Some(
                "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .to_owned(),
            ),
            browser_download_url: crate::branding::fork_download_url(name),
        };
        let artifact =
            fork_artifact_from_asset(&good, "x86_64", "exe", "1.0.0").expect("pinned asset");
        assert_eq!(artifact.version, "9.9.9");
        assert_eq!(artifact.size, 42);
        assert_eq!(
            artifact.sha256,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );

        let mut swapped = good;
        swapped.browser_download_url =
            "https://github.com/rustdesk/rustdesk/releases/download/9.9.9/rustdesk-9.9.9-x86_64.exe"
                .to_owned();
        assert!(fork_artifact_from_asset(&swapped, "x86_64", "exe", "1.0.0").is_err());
    }
}
