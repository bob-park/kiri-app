//! Handling of output files and job packages.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// A non-clashing path in dir. "a.mp4" → "a (1).mp4" → "a (2).mp4" …
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

/// The highest n of `{stem}-{n}.*` in dir (n ≥ 1, any extension, directories included) + 1, or 1.
pub fn next_variant_index(dir: &Path, stem: &str) -> u32 {
    let prefix = format!("{stem}-");
    let max = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let rest = name.strip_prefix(&prefix)?;
            let num = rest.split_once('.').map_or(rest, |(n, _)| n);
            (!num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()))
                .then(|| num.parse::<u32>().ok())
                .flatten()
        })
        .max()
        .unwrap_or(0);
    max.saturating_add(1)
}

/// `dir/{stem}-{index}.{ext}`. If that path already exists (file systems that ignore case
/// or Unicode normalization) it moves to the next number instead of overwriting.
pub fn variant_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let mut i = next_variant_index(dir, stem);
    loop {
        let p = dir.join(format!("{stem}-{i}.{ext}"));
        if !p.exists() || i == u32::MAX {
            return p;
        }
        i += 1;
    }
}

/// rename, or copy + delete across volumes.
pub fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            fs::copy(from, to)?;
            fs::remove_file(from)
        }
        r => r,
    }
}

/// Marker inside a `.kiripart` package. Only packages with it count as kiri's and get removed.
pub const PART_MARKER: &str = ".kiri";
pub const PART_EXT: &str = "kiripart";

/// A title usable as a file name: `/` and `:` become `_`, surrounding spaces and leading dots go, max 120 chars.
pub fn safe_title(title: &str) -> String {
    let t: String = title
        .trim()
        .trim_start_matches('.')
        .trim()
        .chars()
        .map(|c| if c == '/' || c == ':' { '_' } else { c })
        .take(120)
        .collect();
    let t = t.trim_end();
    if t.is_empty() {
        "video".into()
    } else {
        t.into()
    }
}

/// Creates the package folder and its marker file.
pub fn make_part_dir(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join(PART_MARKER), b"")
}

/// Whether it is kiri's package: kiripart extension, a real folder (not a symlink), marker inside.
pub fn is_part_dir(dir: &Path) -> bool {
    dir.extension().is_some_and(|x| x == PART_EXT)
        && fs::symlink_metadata(dir).is_ok_and(|m| m.is_dir())
        && fs::symlink_metadata(dir.join(PART_MARKER)).is_ok_and(|m| m.is_file())
}

/// Removes a package. It lives in the user's folder, so only `is_part_dir` ones go. True if removed.
pub fn remove_part_dir(dir: &Path) -> bool {
    is_part_dir(dir) && fs::remove_dir_all(dir).is_ok()
}

/// Creates the folder and actually writes a test file.
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

/// The output media in a job folder (the first file that is not a subtitle or temp file).
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
    fn safe_title_cleans_names() {
        assert_eq!(safe_title("a/b:c"), "a_b_c");
        assert_eq!(safe_title("  ..hidden  "), "hidden");
        assert_eq!(safe_title(" . "), "video");
        assert_eq!(safe_title(""), "video");
        assert_eq!(safe_title("Rust in 100 Seconds"), "Rust in 100 Seconds");
        let long = "가".repeat(200);
        assert_eq!(safe_title(&long), "가".repeat(120));
    }

    #[test]
    fn make_part_dir_writes_marker() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("x/a.kiripart");
        make_part_dir(&p).unwrap();
        assert!(p.join(PART_MARKER).is_file());
        assert_eq!(find_media(&p).unwrap(), None);
    }

    #[test]
    fn remove_part_dir_only_removes_marked_real_kiripart_dirs() {
        let d = tempfile::tempdir().unwrap();
        let marked = d.path().join("a.kiripart");
        make_part_dir(&marked).unwrap();
        let unmarked = d.path().join("b.kiripart");
        fs::create_dir(&unmarked).unwrap();
        let other = d.path().join("c");
        make_part_dir(&other).unwrap();
        let link = d.path().join("l.kiripart");
        std::os::unix::fs::symlink(&marked, &link).unwrap();
        assert!(!remove_part_dir(&unmarked) && unmarked.exists());
        assert!(!remove_part_dir(&other) && other.exists());
        assert!(!remove_part_dir(&link) && marked.exists());
        assert!(!remove_part_dir(&d.path().join("missing.kiripart")));
        assert!(remove_part_dir(&marked) && !marked.exists());
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

    #[test]
    fn variant_index_counts_any_extension_and_dirs() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(next_variant_index(d.path(), "제목"), 1);
        assert_eq!(
            variant_path(d.path(), "제목", "mp4"),
            d.path().join("제목-1.mp4")
        );
        fs::write(d.path().join("제목.webm"), "").unwrap();
        fs::write(d.path().join("제목-1.mp4"), "").unwrap();
        fs::create_dir(d.path().join("제목-3.kiripart")).unwrap(); // in-progress conversions count too
        assert_eq!(next_variant_index(d.path(), "제목"), 4);
        assert_eq!(
            variant_path(d.path(), "제목", "mov"),
            d.path().join("제목-4.mov")
        );
    }

    #[test]
    fn variant_index_saturates_instead_of_overflowing() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("t-4294967295.mp4"), "").unwrap();
        assert_eq!(next_variant_index(d.path(), "t"), u32::MAX);
    }

    #[test]
    fn variant_path_skips_existing_case_insensitive_match() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("title-1.mp4"), "keep").unwrap();
        if !d.path().join("TITLE-1.mp4").exists() {
            return; // no clash on a case-sensitive file system
        }
        // read_dir sees only "title-1", so the "TITLE" prefix misses it. It must still not overwrite.
        let p = variant_path(d.path(), "TITLE", "mp4");
        assert!(!p.exists(), "{p:?}");
        assert_eq!(p, d.path().join("TITLE-2.mp4"));
    }

    #[test]
    fn variant_index_ignores_lookalike_names() {
        let d = tempfile::tempdir().unwrap();
        for n in [
            "제목2-5.mp4",
            "제목-1a.mp4",
            "제목-.mp4",
            "제목-x-9.mp4",
            "other-7.mp4",
        ] {
            fs::write(d.path().join(n), "").unwrap();
        }
        assert_eq!(next_variant_index(d.path(), "제목"), 1);
        fs::write(d.path().join("a-b-3.mp4"), "").unwrap(); // a '-' in the stem is fine
        assert_eq!(next_variant_index(d.path(), "a-b"), 4);
        assert_eq!(next_variant_index(d.path(), "a"), 1);
    }
}
