//! Downloads yt-dlp and Deno from GitHub Releases, verifies SHA256 and installs them.
//! Files fetched with reqwest carry no quarantine attribute, so Gatekeeper does not block them.
use crate::runner;
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
}
