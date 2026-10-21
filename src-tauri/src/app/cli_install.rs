//! The /usr/local/bin/kiri → bundled kiri-cli symlink, and its commands.
use crate::{
    app::{ipc_server::IpcState, sidecar},
    shared::error::{AppError, AppResult},
};
use serde::Serialize;
use std::{fs, io, path::Path, process::Command};
use tauri::State;

pub const LINK: &str = "/usr/local/bin/kiri";

#[derive(Serialize, Debug)]
pub struct CliStatus {
    pub installed: bool,
    pub link: String,
    pub target: String,
    pub socket_error: Option<String>,
}

pub fn is_installed(target: &Path, link: &Path) -> bool {
    fs::read_link(link).is_ok_and(|p| p == target)
}

/// A path for an sh command inside an AppleScript string. Quotes and backslashes are rejected.
pub fn shell_quote(p: &Path) -> Result<String, String> {
    let s = p.to_str().ok_or("non-UTF-8 path")?;
    if s.contains(['\'', '"', '\\']) {
        return Err(format!("unsupported path: {s}"));
    }
    Ok(format!("'{s}'"))
}

/// The user pressed "Cancel" in the osascript administrator password prompt.
fn is_cancel(stderr: &str) -> bool {
    stderr.contains("(-128)")
}

/// A cancelled password prompt counts as nothing having happened.
fn admin(sh: &str) -> Result<(), String> {
    let script = format!("do shell script \"{sh}\" with administrator privileges");
    let out = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| e.to_string())?;
    let err = String::from_utf8_lossy(&out.stderr);
    if out.status.success() || is_cancel(&err) {
        Ok(())
    } else {
        Err(err.trim().to_string())
    }
}

fn needs_admin(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::PermissionDenied | io::ErrorKind::NotFound | io::ErrorKind::AlreadyExists
    )
}

/// Whether an existing link is kiri's: it points at `target`, or is a stale kiri-cli link
/// left behind by moving the app.
fn is_ours(target: &Path, link: &Path) -> bool {
    fs::read_link(link).is_ok_and(|p| p == target || p.file_name() == Some("kiri-cli".as_ref()))
}

pub fn install(target: &Path, link: &Path) -> Result<(), String> {
    if fs::symlink_metadata(link).is_ok() {
        if !is_ours(target, link) {
            return Err(format!(
                "existing {} is not managed by kiri",
                link.display()
            ));
        }
        if let Err(e) = fs::remove_file(link)
            && e.kind() != io::ErrorKind::NotFound
        {
            return admin_link(target, link, &e);
        }
    }
    match std::os::unix::fs::symlink(target, link) {
        Ok(()) => Ok(()),
        Err(e) => admin_link(target, link, &e),
    }
}

fn admin_link(target: &Path, link: &Path, e: &io::Error) -> Result<(), String> {
    if !needs_admin(e) {
        return Err(e.to_string());
    }
    let dir = link.parent().ok_or("link has no parent")?;
    admin(&format!(
        "mkdir -p {} && ln -sfn {} {}",
        shell_quote(dir)?,
        shell_quote(target)?,
        shell_quote(link)?
    ))
}

/// Removes only kiri's link. Links pointing elsewhere and regular files are left alone.
pub fn uninstall(target: &Path, link: &Path) -> Result<(), String> {
    if fs::symlink_metadata(link).is_err() {
        return Ok(());
    }
    if !is_installed(target, link) {
        return Err(format!("{} is not installed by kiri", link.display()));
    }
    match fs::remove_file(link) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            admin(&format!("rm -f {}", shell_quote(link)?))
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn cli_status(ipc: State<'_, IpcState>) -> AppResult<CliStatus> {
    let target = sidecar("kiri-cli");
    Ok(CliStatus {
        installed: is_installed(&target, Path::new(LINK)),
        link: LINK.into(),
        target: target.display().to_string(),
        socket_error: ipc.0.lock().unwrap().clone(),
    })
}

/// Runs on a blocking thread so the osascript password prompt never blocks the main thread.
async fn blocking(f: impl FnOnce() -> Result<(), String> + Send + 'static) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Failed(e.to_string()))?
        .map_err(AppError::Failed)
}

#[tauri::command]
pub async fn install_cli() -> AppResult<()> {
    blocking(|| install(&sidecar("kiri-cli"), Path::new(LINK))).await
}

#[tauri::command]
pub async fn uninstall_cli() -> AppResult<()> {
    blocking(|| uninstall(&sidecar("kiri-cli"), Path::new(LINK))).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_and_uninstall_in_writable_dir() {
        let d = tempfile::tempdir().unwrap();
        let target = d.path().join("kiri-cli");
        fs::write(&target, "").unwrap();
        let link = d.path().join("bin/kiri");
        fs::create_dir(d.path().join("bin")).unwrap();
        assert!(!is_installed(&target, &link));
        install(&target, &link).unwrap();
        assert!(is_installed(&target, &link));
        install(&target, &link).unwrap(); // reinstalling is fine
        uninstall(&target, &link).unwrap();
        assert!(!is_installed(&target, &link));
        uninstall(&target, &link).unwrap(); // missing link succeeds
    }

    #[test]
    fn uninstall_refuses_foreign_link() {
        let d = tempfile::tempdir().unwrap();
        let link = d.path().join("kiri");
        std::os::unix::fs::symlink("/somewhere/else", &link).unwrap();
        assert!(uninstall(&d.path().join("kiri-cli"), &link).is_err());
        assert!(fs::symlink_metadata(&link).is_ok());
    }

    #[test]
    fn install_replaces_stale_kiri_cli_link() {
        let d = tempfile::tempdir().unwrap();
        let target = d.path().join("kiri-cli");
        let link = d.path().join("kiri");
        std::os::unix::fs::symlink("/Old/kiri.app/Contents/MacOS/kiri-cli", &link).unwrap();
        install(&target, &link).unwrap();
        assert!(is_installed(&target, &link));
    }

    #[test]
    fn install_refuses_foreign_link() {
        let d = tempfile::tempdir().unwrap();
        let link = d.path().join("kiri");
        std::os::unix::fs::symlink("/somewhere/else", &link).unwrap();
        assert!(install(&d.path().join("kiri-cli"), &link).is_err());
        assert_eq!(fs::read_link(&link).unwrap(), Path::new("/somewhere/else"));
    }

    #[test]
    fn install_refuses_regular_file() {
        let d = tempfile::tempdir().unwrap();
        let link = d.path().join("kiri");
        fs::write(&link, "mine").unwrap();
        assert!(install(&d.path().join("kiri-cli"), &link).is_err());
        assert_eq!(fs::read_to_string(&link).unwrap(), "mine");
    }

    #[test]
    fn detects_user_cancel() {
        assert!(is_cancel("0:84: execution error: User canceled. (-128)"));
        assert!(!is_cancel("0:84: execution error: Permission denied (1)"));
    }

    #[test]
    fn link_to_other_target_is_not_installed() {
        let d = tempfile::tempdir().unwrap();
        let link = d.path().join("kiri");
        std::os::unix::fs::symlink("/somewhere/else", &link).unwrap();
        assert!(!is_installed(&d.path().join("kiri-cli"), &link));
    }

    #[test]
    fn shell_quote_rejects_dangerous_paths() {
        assert_eq!(
            shell_quote(Path::new("/Applications/kiri.app/Contents/MacOS/kiri-cli")).unwrap(),
            "'/Applications/kiri.app/Contents/MacOS/kiri-cli'"
        );
        assert!(shell_quote(Path::new("/tmp/a'b")).is_err());
        assert!(shell_quote(Path::new("/tmp/a\"b")).is_err());
        assert!(shell_quote(Path::new("/tmp/a\\b")).is_err());
    }
}
