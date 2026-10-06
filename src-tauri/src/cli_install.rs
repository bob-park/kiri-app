//! /usr/local/bin/kiri → 번들 안 kiri-cli 심볼릭 링크.
use serde::Serialize;
use std::{fs, io, path::Path, process::Command};

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

/// AppleScript 문자열 안의 sh 명령에 넣을 경로. 따옴표·역슬래시가 있으면 거부한다.
pub fn shell_quote(p: &Path) -> Result<String, String> {
    let s = p.to_str().ok_or("non-UTF-8 path")?;
    if s.contains(['\'', '"', '\\']) {
        return Err(format!("unsupported path: {s}"));
    }
    Ok(format!("'{s}'"))
}

fn admin(sh: &str) -> Result<(), String> {
    let script = format!("do shell script \"{sh}\" with administrator privileges");
    let out = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn needs_admin(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::PermissionDenied | io::ErrorKind::NotFound
    )
}

pub fn install(target: &Path, link: &Path) -> Result<(), String> {
    let _ = fs::remove_file(link);
    match std::os::unix::fs::symlink(target, link) {
        Ok(()) => Ok(()),
        Err(e) if needs_admin(&e) => {
            let dir = link.parent().ok_or("link has no parent")?;
            admin(&format!(
                "mkdir -p {} && ln -sf {} {}",
                shell_quote(dir)?,
                shell_quote(target)?,
                shell_quote(link)?
            ))
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn uninstall(link: &Path) -> Result<(), String> {
    match fs::remove_file(link) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            admin(&format!("rm -f {}", shell_quote(link)?))
        }
        Err(e) => Err(e.to_string()),
    }
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
        install(&target, &link).unwrap(); // 다시 설치해도 된다
        uninstall(&link).unwrap();
        assert!(!is_installed(&target, &link));
        uninstall(&link).unwrap(); // 없으면 성공
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
