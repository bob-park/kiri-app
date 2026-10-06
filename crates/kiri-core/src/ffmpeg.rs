//! 인코딩 프리셋 → ffmpeg 인자. VideoToolbox(가속) 와 소프트웨어 두 경로를 가진다.
use crate::model::Preset;
use std::path::Path;

const AAC_MP4: &[&str] = &["-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart"];

/// VideoToolbox 인코더가 있는 프리셋.
pub fn hw_capable(p: Preset) -> bool {
    matches!(p, Preset::Mp4H264 | Preset::Mp4Hevc | Preset::MovProres)
}

pub fn encode_args(p: Preset, hw: bool, input: &Path, output: &Path) -> Option<Vec<String>> {
    let (video, audio): (&[&str], &[&str]) = match (p, hw) {
        (Preset::Original, _) => return None,
        (Preset::Mp4H264, true) => (&["-c:v", "h264_videotoolbox", "-q:v", "65"], AAC_MP4),
        (Preset::Mp4H264, false) => (
            &["-c:v", "libx264", "-crf", "20", "-preset", "medium"],
            AAC_MP4,
        ),
        (Preset::Mp4Hevc, true) => (
            &["-c:v", "hevc_videotoolbox", "-q:v", "60", "-tag:v", "hvc1"],
            AAC_MP4,
        ),
        (Preset::Mp4Hevc, false) => (
            &["-c:v", "libx265", "-crf", "24", "-tag:v", "hvc1"],
            AAC_MP4,
        ),
        (Preset::MovProres, true) => (
            &["-c:v", "prores_videotoolbox", "-profile:v", "2"],
            &["-c:a", "pcm_s16le"],
        ),
        (Preset::MovProres, false) => (
            &["-c:v", "prores_ks", "-profile:v", "2"],
            &["-c:a", "pcm_s16le"],
        ),
        // 맥에는 VP9 하드웨어 인코더가 없다. 디코딩만 가속한다.
        (Preset::WebmVp9, _) => (
            &[
                "-c:v",
                "libvpx-vp9",
                "-crf",
                "32",
                "-b:v",
                "0",
                "-row-mt",
                "1",
            ],
            &["-c:a", "libopus"],
        ),
        (Preset::Mp3, _) => (&["-vn"], &["-c:a", "libmp3lame", "-q:a", "2"]),
        (Preset::M4a, _) => (&["-vn"], &["-c:a", "aac", "-b:a", "192k"]),
    };
    let mut a: Vec<String> = vec!["-hide_banner".into(), "-y".into()];
    if hw && !p.is_audio_only() {
        // 지원하지 않는 코덱(AV1 등)이면 ffmpeg 가 알아서 소프트웨어 디코딩한다.
        a.extend(["-hwaccel".into(), "videotoolbox".into()]);
    }
    a.extend(["-i".into(), input.display().to_string()]);
    a.extend(video.iter().chain(audio).map(|s| s.to_string()));
    a.extend(["-progress".into(), "pipe:1".into(), "-nostats".into()]);
    a.push(output.display().to_string());
    Some(a)
}

/// `-progress pipe:1` 의 key=value 줄. 길이를 모르면 진행률도 모른다.
pub fn parse_progress(line: &str, duration_secs: Option<f64>) -> Option<f32> {
    let (k, v) = line.trim().split_once('=')?;
    match k {
        "progress" if v == "end" => Some(1.0),
        "out_time_us" => {
            let d = duration_secs.filter(|d| *d > 0.0)?;
            let us: f64 = v.parse().ok()?;
            Some(((us / 1_000_000.0) / d).clamp(0.0, 1.0) as f32)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(p: Preset, hw: bool) -> Vec<String> {
        encode_args(p, hw, Path::new("/w/in.webm"), Path::new("/w/out/in.mp4")).unwrap()
    }

    #[test]
    fn original_has_no_encode_step() {
        assert!(encode_args(Preset::Original, true, Path::new("a"), Path::new("b")).is_none());
    }

    #[test]
    fn h264_hw_and_sw() {
        let hw = args(Preset::Mp4H264, true).join(" ");
        assert!(hw.contains("-hwaccel videotoolbox -i /w/in.webm"), "{hw}");
        assert!(hw.contains("-c:v h264_videotoolbox -q:v 65"), "{hw}");
        assert!(
            hw.contains("-c:a aac -b:a 192k -movflags +faststart"),
            "{hw}"
        );
        let sw = args(Preset::Mp4H264, false).join(" ");
        assert!(!sw.contains("hwaccel"), "{sw}");
        assert!(sw.contains("-c:v libx264 -crf 20 -preset medium"), "{sw}");
    }

    #[test]
    fn hevc_and_prores_paths() {
        assert!(
            args(Preset::Mp4Hevc, true)
                .join(" ")
                .contains("-c:v hevc_videotoolbox -q:v 60 -tag:v hvc1")
        );
        assert!(
            args(Preset::Mp4Hevc, false)
                .join(" ")
                .contains("-c:v libx265 -crf 24 -tag:v hvc1")
        );
        assert!(
            args(Preset::MovProres, true)
                .join(" ")
                .contains("-c:v prores_videotoolbox -profile:v 2 -c:a pcm_s16le")
        );
        assert!(
            args(Preset::MovProres, false)
                .join(" ")
                .contains("-c:v prores_ks -profile:v 2")
        );
    }

    #[test]
    fn vp9_is_software_even_with_hw() {
        let a = args(Preset::WebmVp9, true).join(" ");
        assert!(
            a.contains("-c:v libvpx-vp9 -crf 32 -b:v 0 -row-mt 1 -c:a libopus"),
            "{a}"
        );
        assert!(!hw_capable(Preset::WebmVp9));
        assert!(
            hw_capable(Preset::Mp4H264)
                && hw_capable(Preset::Mp4Hevc)
                && hw_capable(Preset::MovProres)
        );
    }

    #[test]
    fn audio_presets_drop_video_and_skip_hwaccel() {
        let mp3 = args(Preset::Mp3, true).join(" ");
        assert!(
            !mp3.contains("hwaccel") && mp3.contains("-vn -c:a libmp3lame -q:a 2"),
            "{mp3}"
        );
        assert!(
            args(Preset::M4a, false)
                .join(" ")
                .contains("-vn -c:a aac -b:a 192k")
        );
    }

    #[test]
    fn progress_flags_and_output_last() {
        let a = args(Preset::Mp4H264, false);
        assert_eq!(&a[..2], ["-hide_banner", "-y"]);
        assert!(
            a.join(" ")
                .contains("-progress pipe:1 -nostats /w/out/in.mp4")
        );
        assert_eq!(a.last().unwrap(), "/w/out/in.mp4");
    }

    #[test]
    fn parses_progress() {
        assert_eq!(
            parse_progress("out_time_us=72000000", Some(144.0)),
            Some(0.5)
        );
        assert_eq!(
            parse_progress("out_time_us=999000000", Some(144.0)),
            Some(1.0)
        );
        assert_eq!(parse_progress("progress=end", None), Some(1.0));
        assert_eq!(parse_progress("out_time_us=N/A", Some(10.0)), None);
        assert_eq!(parse_progress("out_time_us=100", None), None);
        assert_eq!(parse_progress("frame=10", Some(10.0)), None);
    }
}
