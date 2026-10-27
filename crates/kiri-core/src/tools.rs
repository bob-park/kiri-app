//! Downloads yt-dlp and Deno from GitHub Releases and FFmpeg (ffmpeg + ffprobe) from
//! ffmpeg.martin-riedl.de, verifies SHA256 and installs them.
//! Files fetched with reqwest carry no quarantine attribute, so Gatekeeper does not block them.
use crate::{model::Tools, runner};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io, os::unix::fs::PermissionsExt, path::Path, time::Duration};

const YTDLP_ASSET: &str = "yt-dlp_macos";
const YTDLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos";
const YTDLP_SUMS_URL: &str =
    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS";
const DENO_ASSET: &str = "deno-aarch64-apple-darwin.zip";
const DENO_URL: &str =
    "https://github.com/denoland/deno/releases/latest/download/deno-aarch64-apple-darwin.zip";
const DENO_SUM_URL: &str = "https://github.com/denoland/deno/releases/latest/download/deno-aarch64-apple-darwin.zip.sha256sum";

/// Timeouts, so a stalled connection never holds the first-run install forever.
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("kiri/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(300)) // long enough for a full yt-dlp/Deno download on a slow line
        .https_only(true)
        .build()
        .expect("static client config")
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// The hash for `file` from "<hash>  <file>" lines. A file with just one hash line (single .sha256sum) yields that hash.
pub fn find_hash(sums: &str, file: &str) -> Option<String> {
    let lines: Vec<&str> = sums
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    for line in &lines {
        let hash: String = line.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        let name = line[hash.len()..].trim_start_matches(|c: char| c.is_whitespace() || c == '*');
        if is_hex64(&hash) && (name == file || (name.is_empty() && lines.len() == 1)) {
            return Some(hash.to_ascii_lowercase());
        }
    }
    None
}

pub fn install_executable(dest: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = dest.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
    fs::rename(tmp, dest) // a running process keeps the old file (inode) it opened
}

async fn fetch(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("{url}: {e}"))?;
    let resp = resp.error_for_status().map_err(|e| format!("{url}: {e}"))?;
    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("{url}: {e}"))
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    String::from_utf8(fetch(client, url).await?).map_err(|e| format!("{url}: {e}"))
}

/// Does nothing if the installed file's hash matches the latest SHA2-256SUMS.
pub async fn ensure_ytdlp(client: &reqwest::Client, dest: &Path) -> Result<bool, String> {
    let want = ytdlp_latest_hash(client).await?;
    if ytdlp_matches(dest, &want) {
        return Ok(false);
    }
    install_ytdlp(client, dest, &want).await?;
    Ok(true)
}

/// The SHA256 of the latest yt-dlp release (network check only, no download).
pub async fn ytdlp_latest_hash(client: &reqwest::Client) -> Result<String, String> {
    let sums = fetch_text(client, YTDLP_SUMS_URL).await?;
    find_hash(&sums, YTDLP_ASSET).ok_or_else(|| "yt-dlp checksum not found".into())
}

/// Whether the installed yt-dlp already has hash `want`.
pub fn ytdlp_matches(dest: &Path, want: &str) -> bool {
    fs::read(dest).is_ok_and(|current| sha256_hex(&current) == want)
}

/// Downloads yt-dlp, verifies it against `want` and installs it at `dest`.
pub async fn install_ytdlp(
    client: &reqwest::Client,
    dest: &Path,
    want: &str,
) -> Result<(), String> {
    let bytes = fetch(client, YTDLP_URL).await?;
    if sha256_hex(&bytes) != want {
        return Err("yt-dlp checksum mismatch".into());
    }
    install_executable(dest, &bytes).map_err(|e| e.to_string())
}

/// ponytail: installs only when missing. Add updating once yt-dlp ever needs a newer Deno.
pub async fn ensure_deno(client: &reqwest::Client, dest: &Path) -> Result<bool, String> {
    if dest.exists() {
        return Ok(false);
    }
    let sum = fetch_text(client, DENO_SUM_URL).await?;
    let want = find_hash(&sum, DENO_ASSET).ok_or("deno checksum not found")?;
    let zip = fetch(client, DENO_URL).await?;
    if sha256_hex(&zip) != want {
        return Err("deno checksum mismatch".into());
    }
    let dir = dest.parent().ok_or("deno dest has no parent")?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let zip_path = dir.join("deno.zip");
    fs::write(&zip_path, &zip).map_err(|e| e.to_string())?;
    let status = tokio::process::Command::new("/usr/bin/ditto")
        .args(["-x", "-k"])
        .arg(&zip_path)
        .arg(dir)
        .status()
        .await
        .map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&zip_path);
    if !status.success() || !dest.exists() {
        return Err("deno unzip failed".into());
    }
    fs::set_permissions(dest, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    Ok(true)
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ToolName {
    #[serde(rename = "yt-dlp")]
    Ytdlp,
    #[serde(rename = "ffmpeg")]
    Ffmpeg,
    #[serde(rename = "deno")]
    Deno,
}

pub const ALL_TOOLS: [ToolName; 3] = [ToolName::Ytdlp, ToolName::Ffmpeg, ToolName::Deno];

/// Only these release-download prefixes are ever fetched.
const ALLOWED_URL_PREFIXES: [&str; 3] = [
    "https://github.com/yt-dlp/yt-dlp/releases/download/",
    "https://github.com/denoland/deno/releases/download/",
    "https://ffmpeg.martin-riedl.de/download/",
];

#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    pub url: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub version: String,
    pub assets: Vec<Asset>,
}

impl Release {
    /// Numeric version, at least one asset, allowed urls and 64-hex hashes only.
    pub fn valid(&self) -> bool {
        is_numeric_version(&self.version)
            && !self.assets.is_empty()
            && self.assets.iter().all(|a| {
                ALLOWED_URL_PREFIXES.iter().any(|p| a.url.starts_with(p)) && is_hex64(&a.sha256)
            })
    }
}

fn is_numeric_version(v: &str) -> bool {
    v.starts_with(|c: char| c.is_ascii_digit()) && v.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// The version from a tool's `--version` output (first numeric token of the first line).
pub fn parse_version(stdout: &str) -> Option<String> {
    let token = stdout
        .lines()
        .next()?
        .split_whitespace()
        .find(|t| t.starts_with(|c: char| c.is_ascii_digit()))?;
    let run: String = token
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    Some(run.trim_end_matches('.').to_string()).filter(|v| !v.is_empty())
}

/// The version from a GitHub `/releases/tag/<v?ver>` url; pre-release tags yield `None`.
pub fn parse_tag(location: &str) -> Option<String> {
    let (_, tag) = location.split_once("/releases/tag/")?;
    let ver = tag.strip_prefix('v').unwrap_or(tag);
    is_numeric_version(ver).then(|| ver.to_string())
}

/// The version from `…/download/macos/arm64/<id>_<ver>/<name>.zip`.
pub fn parse_riedl_version(url: &str) -> Option<String> {
    if !url.contains("/download/") {
        return None;
    }
    let mut segments = url.rsplit('/');
    segments.next()?;
    let (_, ver) = segments.next()?.rsplit_once('_')?;
    is_numeric_version(ver).then(|| ver.to_string())
}

/// ffmpeg and ffprobe ship as separate zips; they form one release only at the same version.
pub fn ffmpeg_release(ffmpeg: Asset, ffprobe: Asset) -> Option<Release> {
    let version = parse_riedl_version(&ffmpeg.url)?;
    (parse_riedl_version(&ffprobe.url)? == version).then(|| Release {
        version,
        assets: vec![ffmpeg, ffprobe],
    })
}

/// Extracts `bin` from `zip` into a staging dir next to `dest`, then renames it onto `dest`,
/// so a running process keeps the old inode. The staging dir is removed either way.
pub async fn unpack_zip(zip: &[u8], bin: &str, dest: &Path) -> Result<(), String> {
    let dir = dest.parent().ok_or("dest has no parent")?;
    let staging = dir.join(format!(".staging-{bin}"));
    let _ = fs::remove_dir_all(&staging); // leftovers from an interrupted install
    let result = unpack_staged(zip, bin, dest, &staging).await;
    let _ = fs::remove_dir_all(&staging);
    result
}

async fn unpack_staged(zip: &[u8], bin: &str, dest: &Path, staging: &Path) -> Result<(), String> {
    fs::create_dir_all(staging).map_err(|e| e.to_string())?;
    let zip_path = staging.join(format!("{bin}.zip"));
    fs::write(&zip_path, zip).map_err(|e| e.to_string())?;
    let status = tokio::process::Command::new("/usr/bin/ditto")
        .args(["-x", "-k"])
        .arg(&zip_path)
        .arg(staging)
        .status()
        .await
        .map_err(|e| e.to_string())?;
    let staged = staging.join(bin);
    if !status.success() || !staged.is_file() {
        return Err(format!("{bin} unzip failed"));
    }
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    fs::rename(&staged, dest).map_err(|e| e.to_string())
}

const YTDLP_LATEST: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest";
const DENO_LATEST: &str = "https://github.com/denoland/deno/releases/latest";
const FFMPEG_LATEST: &str = "https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release";

/// Like `http_client()`, but never follows redirects, so `latest` can read `Location`.
pub fn redirect_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("kiri/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("static client config")
}

/// The absolute `Location` of a 3xx GET (HEAD 404s on martin-riedl.de); the body is never read.
async fn location(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("{url}: {e}"))?;
    if !resp.status().is_redirection() {
        return Err(format!("{url}: expected a redirect, got {}", resp.status()));
    }
    let loc = resp
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| format!("{url}: no Location header"))?;
    // martin-riedl.de answers with a relative path
    resp.url()
        .join(loc)
        .map(String::from)
        .map_err(|e| format!("{url}: bad Location {loc}: {e}"))
}

/// The version a GitHub `releases/latest` redirect points at.
async fn github_latest(redirects: &reqwest::Client, url: &str) -> Result<String, String> {
    let loc = location(redirects, url).await?;
    parse_tag(&loc).ok_or_else(|| format!("unexpected release tag: {loc}"))
}

/// One asset whose hash comes from `sums_url`, looked up under `file`.
async fn asset(
    client: &reqwest::Client,
    url: String,
    sums_url: &str,
    file: &str,
) -> Result<Asset, String> {
    let sums = fetch_text(client, sums_url).await?;
    let sha256 = find_hash(&sums, file).ok_or_else(|| format!("{file} checksum not found"))?;
    Ok(Asset { url, sha256 })
}

/// The latest stable release of `tool`, pinned to versioned download urls.
pub async fn latest(client: &reqwest::Client, tool: ToolName) -> Result<Release, String> {
    let redirects = redirect_client();
    let release = match tool {
        ToolName::Ytdlp => {
            let version = github_latest(&redirects, YTDLP_LATEST).await?;
            let base = format!("https://github.com/yt-dlp/yt-dlp/releases/download/{version}");
            let a = asset(
                client,
                format!("{base}/{YTDLP_ASSET}"),
                &format!("{base}/SHA2-256SUMS"),
                YTDLP_ASSET,
            )
            .await?;
            Release {
                version,
                assets: vec![a],
            }
        }
        ToolName::Deno => {
            let version = github_latest(&redirects, DENO_LATEST).await?;
            let url = format!(
                "https://github.com/denoland/deno/releases/download/v{version}/{DENO_ASSET}"
            );
            let a = asset(client, url.clone(), &format!("{url}.sha256sum"), DENO_ASSET).await?;
            Release {
                version,
                assets: vec![a],
            }
        }
        ToolName::Ffmpeg => {
            let mut assets = Vec::with_capacity(2);
            for name in ["ffmpeg", "ffprobe"] {
                let file = format!("{name}.zip");
                let url = location(&redirects, &format!("{FFMPEG_LATEST}/{file}")).await?;
                let sums_url = format!("{url}.sha256");
                assets.push(asset(client, url, &sums_url, &file).await?);
            }
            let ffprobe = assets.pop().expect("two assets");
            let ffmpeg = assets.pop().expect("two assets");
            ffmpeg_release(ffmpeg, ffprobe)
                .ok_or("ffmpeg and ffprobe versions differ".to_string())?
        }
    };
    if !release.valid() {
        return Err(format!("{}: invalid release {release:?}", tool_label(tool)));
    }
    Ok(release)
}

fn tool_label(tool: ToolName) -> &'static str {
    match tool {
        ToolName::Ytdlp => "yt-dlp",
        ToolName::Ffmpeg => "ffmpeg",
        ToolName::Deno => "deno",
    }
}

/// Downloads and verifies every asset first, then installs, so a failed download changes nothing.
pub async fn install(
    client: &reqwest::Client,
    tool: ToolName,
    release: &Release,
    paths: &Tools,
) -> Result<(), String> {
    if !release.valid() {
        return Err(format!("{}: invalid release", tool_label(tool)));
    }
    let mut blobs = Vec::with_capacity(release.assets.len());
    for a in &release.assets {
        let bytes = fetch(client, &a.url).await?;
        if sha256_hex(&bytes) != a.sha256 {
            return Err(format!("{} checksum mismatch", tool_label(tool)));
        }
        blobs.push(bytes);
    }
    let targets: &[(&str, &Path)] = match tool {
        ToolName::Ytdlp => {
            return install_executable(&paths.ytdlp, &blobs[0]).map_err(|e| e.to_string());
        }
        ToolName::Deno => &[("deno", &paths.deno)],
        // ffmpeg_release orders the assets ffmpeg, ffprobe
        ToolName::Ffmpeg => &[("ffmpeg", &paths.ffmpeg), ("ffprobe", &paths.ffprobe)],
    };
    if blobs.len() != targets.len() {
        return Err(format!("{}: unexpected asset count", tool_label(tool)));
    }
    for ((bin, dest), zip) in targets.iter().zip(&blobs) {
        if let Some(dir) = dest.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        unpack_zip(zip, bin, dest).await?;
    }
    Ok(())
}

pub fn is_installed(tool: ToolName, paths: &Tools) -> bool {
    match tool {
        ToolName::Ytdlp => paths.ytdlp.is_file(),
        ToolName::Deno => paths.deno.is_file(),
        ToolName::Ffmpeg => paths.ffmpeg.is_file() && paths.ffprobe.is_file(),
    }
}

/// The installed version (ffmpeg's from `ffmpeg -version`), or `None` when missing or unreadable.
pub async fn installed_version(tool: ToolName, paths: &Tools) -> Option<String> {
    let (bin, flag) = match tool {
        ToolName::Ytdlp => (&paths.ytdlp, "--version"),
        ToolName::Deno => (&paths.deno, "--version"),
        ToolName::Ffmpeg => (&paths.ffmpeg, "-version"),
    };
    let out = runner::output(bin, &[flag.to_string()]).await.ok()?;
    parse_version(&out)
}

pub async fn version(bin: &Path) -> Option<String> {
    let out = runner::output(bin, &["--version".to_string()]).await.ok()?;
    out.lines()
        .next()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    const H1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const H2: &str = "2222222222222222222222222222222222222222222222222222222222222222";

    /// The timeout + https_only combination builds (the `expect` never fires).
    #[test]
    fn http_client_builds() {
        let _ = http_client();
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn finds_hash_in_sums_file() {
        let sums = format!("{H1}  yt-dlp\n{H2}  yt-dlp_macos\n{H1}  yt-dlp_macos.zip\n");
        assert_eq!(find_hash(&sums, "yt-dlp_macos").as_deref(), Some(H2));
        assert_eq!(find_hash(&sums, "yt-dlp").as_deref(), Some(H1));
        assert_eq!(find_hash(&sums, "missing"), None);
    }

    #[test]
    fn finds_hash_in_single_file_sum() {
        let zip = "deno-aarch64-apple-darwin.zip";
        assert_eq!(
            find_hash(&format!("{H2}  {zip}\n"), zip).as_deref(),
            Some(H2)
        );
        assert_eq!(find_hash(&format!("{H2}\n"), zip).as_deref(), Some(H2));
        assert_eq!(
            find_hash(&format!("{} *bin\n", H2.to_uppercase()), "bin").as_deref(),
            Some(H2)
        );
        assert_eq!(find_hash("garbage", "x"), None);
    }

    #[test]
    fn install_executable_replaces_atomically_with_mode() {
        let d = tempfile::tempdir().unwrap();
        let dest = d.path().join("bin/yt-dlp");
        install_executable(&dest, b"v1").unwrap();
        install_executable(&dest, b"v2").unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"v2");
        assert_eq!(
            fs::metadata(&dest).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert!(!dest.with_extension("tmp").exists());
    }

    #[tokio::test]
    async fn version_reads_first_line() {
        let d = tempfile::tempdir().unwrap();
        let bin = crate::testutil::script(d.path(), "yt-dlp", "echo 2026.09.30\necho extra");
        assert_eq!(version(&bin).await.as_deref(), Some("2026.09.30"));
        assert_eq!(version(&d.path().join("missing")).await, None);
    }

    fn tools_in(dir: &Path) -> Tools {
        let bin = dir.join("bin");
        Tools {
            ytdlp: bin.join("yt-dlp"),
            deno: bin.join("deno"),
            ffmpeg: bin.join("ffmpeg"),
            ffprobe: bin.join("ffprobe"),
        }
    }

    #[test]
    fn is_installed_ffmpeg_needs_both() {
        let d = tempfile::tempdir().unwrap();
        let paths = tools_in(d.path());
        fs::create_dir_all(paths.ffmpeg.parent().unwrap()).unwrap();
        fs::write(&paths.ffmpeg, "x").unwrap();
        assert!(!is_installed(ToolName::Ffmpeg, &paths));
        fs::write(&paths.ffprobe, "x").unwrap();
        assert!(is_installed(ToolName::Ffmpeg, &paths));
        assert!(!is_installed(ToolName::Ytdlp, &paths));
    }

    #[tokio::test]
    async fn installed_version_runs_the_binary() {
        let d = tempfile::tempdir().unwrap();
        let mut paths = tools_in(d.path());
        assert_eq!(installed_version(ToolName::Ffmpeg, &paths).await, None);
        paths.ffmpeg = crate::testutil::script(
            d.path(),
            "ffmpeg",
            r#"[ "$1" = "-version" ] && echo "ffmpeg version 9.0.2-x""#,
        );
        assert_eq!(
            installed_version(ToolName::Ffmpeg, &paths).await.as_deref(),
            Some("9.0.2")
        );
    }

    /// Network: resolves and installs the real latest release of each tool.
    #[tokio::test]
    #[ignore]
    async fn installs_latest_of_each_tool() {
        let d = tempfile::tempdir().unwrap();
        let paths = tools_in(d.path());
        let client = http_client();
        for tool in ALL_TOOLS {
            let release = latest(&client, tool).await.unwrap();
            println!("{tool:?}: {release:?}");
            install(&client, tool, &release, &paths).await.unwrap();
            assert!(is_installed(tool, &paths));
            assert_eq!(
                installed_version(tool, &paths).await.as_deref(),
                Some(release.version.as_str())
            );
        }
        let encoders = runner::output(
            &paths.ffmpeg,
            &["-hide_banner".to_string(), "-encoders".to_string()],
        )
        .await
        .unwrap();
        let missing: Vec<_> = [
            "libx264",
            "libx265",
            "libvpx-vp9",
            "libmp3lame",
            "libopus",
            "h264_videotoolbox",
            "hevc_videotoolbox",
            "prores_videotoolbox",
            "prores_ks",
        ]
        .into_iter()
        .filter(|e| !encoders.split_whitespace().any(|w| w == *e))
        .collect();
        assert!(missing.is_empty(), "missing encoders: {missing:?}");
    }

    #[test]
    fn parse_version_from_each_tool() {
        assert_eq!(
            parse_version(
                "ffmpeg version 9.0.2-https://www.martin-riedl.de Copyright (c) 2000-2026\n"
            )
            .as_deref(),
            Some("9.0.2")
        );
        assert_eq!(parse_version("2026.10.20\n").as_deref(), Some("2026.10.20"));
        assert_eq!(
            parse_version("deno 2.5.4 (stable, release, aarch64-apple-darwin)\nv8 14.0").as_deref(),
            Some("2.5.4")
        );
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn parse_tag_and_riedl_urls() {
        assert_eq!(
            parse_tag("https://github.com/yt-dlp/yt-dlp/releases/tag/2026.10.20").as_deref(),
            Some("2026.10.20")
        );
        assert_eq!(
            parse_tag("https://github.com/denoland/deno/releases/tag/v2.5.4").as_deref(),
            Some("2.5.4")
        );
        assert_eq!(
            parse_tag("https://github.com/denoland/deno/releases/tag/v2.6.0-rc.1"),
            None
        );
        assert_eq!(
            parse_riedl_version(
                "https://ffmpeg.martin-riedl.de/download/macos/arm64/1789931890_9.0.2/ffmpeg.zip"
            )
            .as_deref(),
            Some("9.0.2")
        );
        assert_eq!(
            parse_riedl_version(
                "https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release/ffmpeg.zip"
            ),
            None
        );
    }

    #[test]
    fn ffmpeg_release_needs_matching_versions() {
        let a = |id_ver: &str, n: &str| Asset {
            url: format!("https://ffmpeg.martin-riedl.de/download/macos/arm64/{id_ver}/{n}.zip"),
            sha256: H1.into(),
        };
        let r = ffmpeg_release(a("1_9.0.2", "ffmpeg"), a("2_9.0.2", "ffprobe")).unwrap();
        assert_eq!(r.version, "9.0.2");
        assert!(r.valid());
        assert!(r.assets[0].url.ends_with("/ffmpeg.zip"));
        assert!(r.assets[1].url.ends_with("/ffprobe.zip"));
        assert_eq!(
            ffmpeg_release(a("1_9.0.2", "ffmpeg"), a("2_9.0.1", "ffprobe")),
            None
        );
    }

    #[test]
    fn release_validity() {
        let ok = Release {
            version: "9.0.2".into(),
            assets: vec![Asset {
                url: "https://ffmpeg.martin-riedl.de/download/x/ffmpeg.zip".into(),
                sha256: H1.into(),
            }],
        };
        assert!(ok.valid());
        let mut bad = ok.clone();
        bad.assets[0].url = "http://ffmpeg.martin-riedl.de/download/x/ffmpeg.zip".into();
        assert!(!bad.valid());
        let mut bad = ok.clone();
        bad.assets[0].url = "https://evil.example/download/ffmpeg.zip".into();
        assert!(!bad.valid());
        let mut bad = ok.clone();
        bad.version = "9.0-dev".into();
        assert!(!bad.valid());
        let mut bad = ok.clone();
        bad.assets[0].sha256 = "abc".into();
        assert!(!bad.valid());
        assert!(
            !Release {
                assets: vec![],
                ..ok
            }
            .valid()
        );
    }

    #[tokio::test]
    async fn unpack_zip_replaces_by_rename() {
        use std::io::Read;
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("tool"), "#!/bin/sh\necho new\n").unwrap();
        fs::write(src.join("other"), "x").unwrap();
        let zip_of = |name: &str| {
            let zip = d.path().join(format!("{name}.zip"));
            let ok = std::process::Command::new("/usr/bin/ditto")
                .args(["-c", "-k"])
                .arg(src.join(name))
                .arg(&zip)
                .status()
                .unwrap()
                .success();
            assert!(ok);
            fs::read(zip).unwrap()
        };

        let bin = d.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let dest = bin.join("tool");
        fs::write(&dest, "old").unwrap();
        let mut old = fs::File::open(&dest).unwrap();

        unpack_zip(&zip_of("tool"), "tool", &dest).await.unwrap();
        assert_eq!(fs::read_to_string(&dest).unwrap(), "#!/bin/sh\necho new\n");
        assert_eq!(
            fs::metadata(&dest).unwrap().permissions().mode() & 0o777,
            0o755
        );
        let mut s = String::new();
        old.read_to_string(&mut s).unwrap();
        assert_eq!(s, "old"); // the open fd still points at the old inode
        assert!(!bin.join(".staging-tool").exists());

        fs::write(&dest, "old").unwrap();
        assert!(unpack_zip(&zip_of("other"), "tool", &dest).await.is_err());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "old");
        assert!(!bin.join(".staging-tool").exists());
    }

    #[test]
    fn tool_name_serde() {
        assert_eq!(
            serde_json::to_value(ToolName::Ytdlp).unwrap(),
            serde_json::json!("yt-dlp")
        );
        assert_eq!(
            serde_json::from_value::<ToolName>(serde_json::json!("ffmpeg")).unwrap(),
            ToolName::Ffmpeg
        );
        assert_eq!(ALL_TOOLS.len(), 3);
    }
}
