//! yt-dlp 호출 인자와 출력 파서. 프로세스 실행은 runner 가 맡는다.
use crate::model::{JobOptions, Tools};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

pub const PROGRESS_TEMPLATE: &str =
    "download:KIRI|%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s";

const HOSTS: [&str; 5] = [
    "youtube.com",
    "www.youtube.com",
    "m.youtube.com",
    "music.youtube.com",
    "youtu.be",
];

/// http(s) 이고 호스트가 YouTube 이며 경로가 있는 URL.
pub fn is_youtube_url(s: &str) -> bool {
    let s = s.trim();
    let Some(rest) = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
    else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let has_path = rest.len() > authority.len() + 1;
    HOSTS.contains(&host.as_str()) && has_path
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Quality {
    pub format_id: String,
    pub height: u32,
    pub fps: Option<u32>,
    pub vcodec: String,
    /// 예상 용량 (비디오 + bestaudio)
    pub filesize: Option<u64>,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub channel: Option<String>,
    pub duration_secs: Option<f64>,
    pub thumbnail: Option<String>,
    /// 높은 화질부터
    pub qualities: Vec<Quality>,
    pub subtitles: Vec<String>,
    pub auto_subtitles: Vec<String>,
}

fn text(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}
fn num(v: &Value, k: &str) -> f64 {
    v[k].as_f64().unwrap_or(0.0)
}
fn size(f: &Value) -> Option<u64> {
    f["filesize"].as_u64().or(f["filesize_approx"].as_u64())
}
fn has_audio(f: &Value) -> bool {
    f["acodec"].as_str().is_some_and(|a| a != "none")
}
fn is_video(f: &Value) -> bool {
    f["vcodec"].as_str().is_some_and(|v| v != "none")
        && f["height"].as_u64().is_some()
        && f["ext"].as_str() != Some("mhtml")
}
fn is_audio(f: &Value) -> bool {
    f["vcodec"].as_str() == Some("none") && has_audio(f)
}
fn label(height: u32, fps: Option<u32>) -> String {
    match fps {
        Some(f) if f > 30 => format!("{height}p{f}"),
        _ => format!("{height}p"),
    }
}
fn short_codec(vcodec: &str) -> String {
    match vcodec.split('.').next().unwrap_or("") {
        "avc1" | "h264" => "AVC".into(),
        "vp09" | "vp9" => "VP9".into(),
        "av01" => "AV1".into(),
        "hev1" | "hvc1" => "HEVC".into(),
        other => other.to_uppercase(),
    }
}
fn langs(v: &Value, auto: bool) -> Vec<String> {
    let mut out: Vec<String> = v
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    out.retain(|k| k != "live_chat" && !(auto && k.ends_with("-orig")));
    out.sort();
    out
}

pub fn parse_probe(json: &str) -> Result<VideoInfo, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("invalid yt-dlp output: {e}"))?;
    let formats = v["formats"].as_array().cloned().unwrap_or_default();
    let audio_size = formats
        .iter()
        .filter(|f| is_audio(f))
        .max_by(|a, b| num(a, "abr").total_cmp(&num(b, "abr")))
        .and_then(size);

    // (해상도, fps) 마다 tbr 가 가장 높은 포맷 하나
    let mut best: Vec<(Quality, f64)> = Vec::new();
    for f in formats.iter().filter(|f| is_video(f)) {
        let height = f["height"].as_u64().unwrap_or(0) as u32;
        let fps = f["fps"].as_f64().map(|x| x.round() as u32);
        let tbr = num(f, "tbr");
        let extra = if has_audio(f) {
            0
        } else {
            audio_size.unwrap_or(0)
        };
        let q = Quality {
            format_id: text(f, "format_id"),
            height,
            fps,
            vcodec: short_codec(f["vcodec"].as_str().unwrap_or("")),
            filesize: size(f).map(|s| s + extra),
            label: label(height, fps),
        };
        match best
            .iter_mut()
            .find(|(b, _)| b.height == height && b.fps == fps)
        {
            Some(slot) if tbr > slot.1 => *slot = (q, tbr),
            Some(_) => {}
            None => best.push((q, tbr)),
        }
    }
    let mut qualities: Vec<Quality> = best.into_iter().map(|(q, _)| q).collect();
    qualities.sort_by(|a, b| b.height.cmp(&a.height).then(b.fps.cmp(&a.fps)));

    Ok(VideoInfo {
        id: text(&v, "id"),
        title: text(&v, "title"),
        channel: v["channel"]
            .as_str()
            .or(v["uploader"].as_str())
            .map(String::from),
        duration_secs: v["duration"].as_f64(),
        thumbnail: v["thumbnail"].as_str().map(String::from),
        qualities,
        subtitles: langs(&v["subtitles"], false),
        auto_subtitles: langs(&v["automatic_captions"], true),
    })
}

/// "best" | "audio" | "<N>p". 같은 높이가 없으면 그보다 낮은 것 중 최고, 그것도 없으면 가장 낮은 것.
/// Ok(None) 은 오디오만.
pub fn resolve_quality(qualities: &[Quality], want: &str) -> Result<Option<Quality>, String> {
    match want {
        "audio" => Ok(None),
        "best" => Ok(qualities.first().cloned()),
        w => {
            let h: u32 = w
                .strip_suffix('p')
                .and_then(|n| n.parse().ok())
                .ok_or_else(|| w.to_string())?;
            Ok(qualities
                .iter()
                .find(|q| q.height <= h)
                .or(qualities.last())
                .cloned())
        }
    }
}

/// 요청 언어 중 수동 자막이 있으면 그것을, 없으면 자동 생성 자막을 고른다. 둘 다 없으면 버린다.
pub fn pick_subtitles(info: &VideoInfo, langs: &[String]) -> (Vec<String>, bool) {
    let mut out = Vec::new();
    let mut auto = false;
    for l in langs {
        if info.subtitles.contains(l) {
            out.push(l.clone());
        } else if info.auto_subtitles.contains(l) {
            out.push(l.clone());
            auto = true;
        }
    }
    (out, auto)
}

fn deno_flag(tools: &Tools) -> String {
    format!("deno:{}", tools.deno.display())
}

pub fn probe_args(url: &str, tools: &Tools) -> Vec<String> {
    vec![
        "-J".into(),
        "--no-playlist".into(),
        "--js-runtimes".into(),
        deno_flag(tools),
        "--".into(),
        url.into(),
    ]
}

pub fn download_args(url: &str, o: &JobOptions, tools: &Tools, out_dir: &Path) -> Vec<String> {
    let format = match &o.format_id {
        Some(id) if !o.preset.is_audio_only() => format!("{id}+bestaudio/{id}"),
        _ => "bestaudio".into(),
    };
    // ffmpeg·ffprobe 가 같은 디렉터리에 있으므로 디렉터리를 넘긴다.
    let ffmpeg_dir = tools
        .ffmpeg
        .parent()
        .unwrap_or(Path::new("/"))
        .display()
        .to_string();
    let mut a: Vec<String> = vec![
        "-f".into(),
        format,
        "--no-playlist".into(),
        "--newline".into(),
        "--color".into(),
        "never".into(),
        "--progress-template".into(),
        PROGRESS_TEMPLATE.into(),
        "--ffmpeg-location".into(),
        ffmpeg_dir,
        "--js-runtimes".into(),
        deno_flag(tools),
    ];
    if !o.subtitles.is_empty() {
        a.push("--write-subs".into());
        if o.auto_subtitles {
            a.push("--write-auto-subs".into());
        }
        a.extend([
            "--sub-langs".into(),
            o.subtitles.join(","),
            "--convert-subs".into(),
            "srt".into(),
        ]);
    }
    a.extend([
        "-o".into(),
        out_dir.join("%(title)s.%(ext)s").display().to_string(),
        "--".into(),
        url.into(),
    ]);
    a
}

#[derive(Clone, Debug, PartialEq)]
pub struct Progress {
    pub fraction: f32,
    pub speed: Option<String>,
    pub eta: Option<String>,
}

/// PROGRESS_TEMPLATE 이 찍은 줄만 해석한다.
pub fn parse_progress(line: &str) -> Option<Progress> {
    let rest = line.trim().strip_prefix("KIRI|")?;
    let mut parts = rest.split('|');
    let pct: f32 = parts
        .next()?
        .trim()
        .trim_end_matches('%')
        .trim()
        .parse()
        .ok()?;
    let clean = |s: Option<&str>| {
        s.map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with("Unknown") && *s != "N/A" && *s != "NA")
            .map(String::from)
    };
    Some(Progress {
        fraction: (pct / 100.0).clamp(0.0, 1.0),
        speed: clean(parts.next()),
        eta: clean(parts.next()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Preset;
    use std::path::PathBuf;

    const FIXTURE: &str = include_str!("../tests/fixtures/probe.json");

    fn tools() -> Tools {
        Tools {
            ytdlp: "/bin/yt-dlp".into(),
            deno: "/bin/deno".into(),
            ffmpeg: "/app/MacOS/ffmpeg".into(),
        }
    }

    #[test]
    fn accepts_youtube_urls() {
        for u in [
            "https://www.youtube.com/watch?v=abc123",
            "https://youtube.com/watch?v=abc123",
            "http://m.youtube.com/watch?v=abc123",
            "https://music.youtube.com/watch?v=abc123",
            "https://youtu.be/abc123",
            "https://www.youtube.com/shorts/abc123",
            "  https://youtu.be/abc123?t=10  ",
        ] {
            assert!(is_youtube_url(u), "{u}");
        }
    }

    #[test]
    fn rejects_other_input() {
        for u in [
            "",
            "hello",
            "youtube.com/watch?v=abc",
            "https://vimeo.com/1",
            "https://youtube.com.evil.com/watch?v=x",
            "https://notyoutube.com/watch?v=x",
            "https://youtube.com/",
            "ftp://youtube.com/watch?v=x",
        ] {
            assert!(!is_youtube_url(u), "{u}");
        }
    }

    #[test]
    fn parses_probe_metadata() {
        let info = parse_probe(FIXTURE).unwrap();
        assert_eq!(info.id, "abc123");
        assert_eq!(info.title, "Rust in 100 Seconds");
        assert_eq!(info.channel.as_deref(), Some("Fireship"));
        assert_eq!(info.duration_secs, Some(144.0));
        assert_eq!(info.subtitles, vec!["en", "ko"]);
        assert_eq!(info.auto_subtitles, vec!["en", "ja", "ko"]);
    }

    #[test]
    fn parses_qualities_best_per_resolution() {
        let q = parse_probe(FIXTURE).unwrap().qualities;
        let ids: Vec<_> = q.iter().map(|q| q.format_id.as_str()).collect();
        assert_eq!(ids, vec!["299", "136", "18"]); // 높은 tbr 승, 스토리보드·오디오 제외
        assert_eq!(q[0].label, "1080p60");
        assert_eq!(q[1].label, "720p");
        assert_eq!(q[0].vcodec, "AVC");
        assert_eq!(q[0].filesize, Some(85_000_000 + 3_000_000)); // 비디오 + bestaudio
        assert_eq!(q[2].filesize, Some(20_000_000)); // 오디오 포함 포맷은 더하지 않음
    }

    #[test]
    fn rejects_garbage_probe_output() {
        assert!(parse_probe("not json").is_err());
    }

    #[test]
    fn resolves_quality() {
        let q = parse_probe(FIXTURE).unwrap().qualities;
        let id = |w: &str| resolve_quality(&q, w).unwrap().map(|q| q.format_id);
        assert_eq!(id("best").as_deref(), Some("299"));
        assert_eq!(id("1080p").as_deref(), Some("299"));
        assert_eq!(id("720p").as_deref(), Some("136"));
        assert_eq!(id("480p").as_deref(), Some("18"));
        assert_eq!(id("144p").as_deref(), Some("18")); // 더 낮은 게 없으면 가장 낮은 것
        assert_eq!(id("audio"), None);
        assert!(resolve_quality(&q, "hd").is_err());
    }

    #[test]
    fn picks_manual_then_auto_subtitles() {
        let info = parse_probe(FIXTURE).unwrap();
        let langs = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            pick_subtitles(&info, &langs(&["ko", "en"])),
            (langs(&["ko", "en"]), false)
        );
        assert_eq!(
            pick_subtitles(&info, &langs(&["ko", "ja", "fr"])),
            (langs(&["ko", "ja"]), true)
        );
        assert_eq!(pick_subtitles(&info, &[]), (vec![], false));
    }

    #[test]
    fn probe_args_use_deno() {
        let a = probe_args("https://youtu.be/x", &tools());
        assert_eq!(
            a,
            vec![
                "-J",
                "--no-playlist",
                "--js-runtimes",
                "deno:/bin/deno",
                "--",
                "https://youtu.be/x"
            ]
        );
    }

    #[test]
    fn download_args_video_with_subs() {
        let o = JobOptions {
            format_id: Some("299".into()),
            preset: Preset::Mp4H264,
            subtitles: vec!["ko".into(), "ja".into()],
            auto_subtitles: true,
        };
        let a = download_args(
            "https://youtu.be/x",
            &o,
            &tools(),
            &PathBuf::from("/c/jobs/1"),
        );
        let s = a.join(" ");
        assert!(s.starts_with("-f 299+bestaudio/299 --no-playlist"), "{s}");
        assert!(s.contains("--ffmpeg-location /app/MacOS"), "{s}");
        assert!(s.contains("--js-runtimes deno:/bin/deno"), "{s}");
        assert!(
            s.contains("--write-subs --write-auto-subs --sub-langs ko,ja --convert-subs srt"),
            "{s}"
        );
        assert!(
            s.contains(&format!("--progress-template {PROGRESS_TEMPLATE}")),
            "{s}"
        );
        assert_eq!(a[a.len() - 3], "/c/jobs/1/%(title)s.%(ext)s");
        assert_eq!(a[a.len() - 2], "--");
        assert_eq!(a.last().unwrap(), "https://youtu.be/x");
    }

    #[test]
    fn download_args_audio_only_without_subs() {
        let o = JobOptions {
            format_id: Some("299".into()),
            preset: Preset::Mp3,
            subtitles: vec![],
            auto_subtitles: false,
        };
        let a = download_args("u", &o, &tools(), Path::new("/d"));
        assert_eq!(&a[..2], ["-f", "bestaudio"]);
        assert!(!a.iter().any(|x| x.starts_with("--write")));
    }

    #[test]
    fn parses_progress_lines() {
        let p = parse_progress("KIRI|  45.3%|   2.10MiB/s|00:12").unwrap();
        assert!((p.fraction - 0.453).abs() < 1e-4);
        assert_eq!(p.speed.as_deref(), Some("2.10MiB/s"));
        assert_eq!(p.eta.as_deref(), Some("00:12"));
        let done = parse_progress("KIRI|100.0%|Unknown B/s|NA").unwrap();
        assert_eq!((done.fraction, done.speed, done.eta), (1.0, None, None));
        assert!(parse_progress("[download] Destination: x.mp4").is_none());
        assert!(parse_progress("KIRI|  N/A%|x|y").is_none());
    }
}
