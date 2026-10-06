//! 다운로드 결과 파일 다루기.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// dir 안에서 겹치지 않는 경로. "a.mp4" → "a (1).mp4" → "a (2).mp4" …
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let p = Path::new(name);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = p.extension().and_then(|e| e.to_str());
    (1..)
        .map(|i| match ext {
            Some(e) => dir.join(format!("{stem} ({i}).{e}")),
            None => dir.join(format!("{stem} ({i})")),
        })
        .find(|c| !c.exists())
        .expect("unbounded range")
}

/// rename, 볼륨이 다르면 복사 후 삭제.
pub fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            fs::copy(from, to)?;
            fs::remove_file(from)
        }
        r => r,
    }
}

/// 폴더를 만들고 실제로 파일을 써 본다.
pub fn check_writable(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let probe = dir.join(".kiri-write-test");
    fs::write(&probe, b"")?;
    fs::remove_file(probe)
}

const AUX_EXT: [&str; 8] = ["srt", "vtt", "part", "ytdl", "json", "jpg", "webp", "png"];

fn is_aux(p: &Path) -> bool {
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    name.starts_with('.') || name.contains(".part-Frag") || AUX_EXT.contains(&ext)
}

/// 작업 폴더에서 결과 미디어(자막·임시 파일이 아닌 첫 파일).
pub fn find_media(dir: &Path) -> io::Result<Option<PathBuf>> {
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_file() && !is_aux(&p) {
            return Ok(Some(p));
        }
    }
    Ok(None)
}

pub fn subtitle_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = vec![];
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("srt") {
            out.push(p);
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn unique_path_returns_name_when_free() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(unique_path(d.path(), "a.mp4"), d.path().join("a.mp4"));
    }

    #[test]
    fn unique_path_appends_counter() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("a.mp4"), "").unwrap();
        assert_eq!(unique_path(d.path(), "a.mp4"), d.path().join("a (1).mp4"));
        fs::write(d.path().join("a (1).mp4"), "").unwrap();
        assert_eq!(unique_path(d.path(), "a.mp4"), d.path().join("a (2).mp4"));
        fs::write(d.path().join("noext"), "").unwrap();
        assert_eq!(unique_path(d.path(), "noext"), d.path().join("noext (1)"));
    }

    #[test]
    fn move_file_moves() {
        let d = tempfile::tempdir().unwrap();
        let (a, b) = (d.path().join("a"), d.path().join("b"));
        fs::write(&a, "x").unwrap();
        move_file(&a, &b).unwrap();
        assert!(!a.exists());
        assert_eq!(fs::read_to_string(b).unwrap(), "x");
    }

    #[test]
    fn check_writable_creates_dir_and_detects_readonly() {
        let d = tempfile::tempdir().unwrap();
        let sub = d.path().join("new/dir");
        check_writable(&sub).unwrap();
        assert!(sub.is_dir());
        let ro = d.path().join("ro");
        fs::create_dir(&ro).unwrap();
        fs::set_permissions(&ro, fs::Permissions::from_mode(0o555)).unwrap();
        assert!(check_writable(&ro).is_err());
        fs::set_permissions(&ro, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn finds_media_and_subtitles() {
        let d = tempfile::tempdir().unwrap();
        for n in ["v.ko.srt", "v.mp4.part", ".hidden", "v.webm"] {
            fs::write(d.path().join(n), "").unwrap();
        }
        fs::create_dir(d.path().join("out")).unwrap();
        assert_eq!(find_media(d.path()).unwrap(), Some(d.path().join("v.webm")));
        assert_eq!(
            subtitle_files(d.path()).unwrap(),
            vec![d.path().join("v.ko.srt")]
        );
    }
}
