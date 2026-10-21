//! Job model shared by the queue and the CLI. Its serde form is the frontend/CLI contract.
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    Original,
    Mp4H264,
    Mp4Hevc,
    MovProres,
    WebmVp9,
    Mp3,
    M4a,
}

impl Preset {
    pub const ALL: [Preset; 7] = [
        Preset::Original,
        Preset::Mp4H264,
        Preset::Mp4Hevc,
        Preset::MovProres,
        Preset::WebmVp9,
        Preset::Mp3,
        Preset::M4a,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Preset::Original => "original",
            Preset::Mp4H264 => "mp4-h264",
            Preset::Mp4Hevc => "mp4-hevc",
            Preset::MovProres => "mov-prores",
            Preset::WebmVp9 => "webm-vp9",
            Preset::Mp3 => "mp3",
            Preset::M4a => "m4a",
        }
    }

    pub fn is_audio_only(self) -> bool {
        matches!(self, Preset::Mp3 | Preset::M4a)
    }

    /// Extension of the re-encoded output. Original is not encoded, so None.
    pub fn extension(self) -> Option<&'static str> {
        match self {
            Preset::Original => None,
            Preset::Mp4H264 | Preset::Mp4Hevc => Some("mp4"),
            Preset::MovProres => Some("mov"),
            Preset::WebmVp9 => Some("webm"),
            Preset::Mp3 => Some("mp3"),
            Preset::M4a => Some("m4a"),
        }
    }
}

impl FromStr for Preset {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        Preset::ALL.into_iter().find(|p| p.id() == s).ok_or(())
    }
}

impl fmt::Display for Preset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// What a job works on: download a YouTube link, or convert a local file.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JobSource {
    Youtube {
        url: String,
    },
    File {
        path: PathBuf,
        /// None = the source file's folder
        #[serde(default)]
        output_dir: Option<PathBuf>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JobOptions {
    /// Video format_id. None downloads audio only.
    pub format_id: Option<String>,
    pub preset: Preset,
    pub subtitles: Vec<String>,
    /// True if `subtitles` includes auto-generated ones (--write-auto-subs).
    pub auto_subtitles: bool,
    /// Max output height. None keeps the source resolution.
    #[serde(default)]
    pub max_height: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Downloading,
    Encoding,
    Completed,
    Failed(String),
    Stopped,
}

impl JobState {
    /// A process is running.
    pub fn is_active(&self) -> bool {
        matches!(self, JobState::Downloading | JobState::Encoding)
    }
    /// Not finished yet (queued included).
    pub fn is_pending(&self) -> bool {
        matches!(self, JobState::Queued) || self.is_active()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Job {
    pub id: u64,
    pub source: JobSource,
    pub title: String,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub duration_secs: Option<f64>,
    /// Quality for display ("1080p60", or "audio" for audio only).
    #[serde(default)]
    pub quality_label: String,
    pub options: JobOptions,
    pub state: JobState,
    /// 0.0–1.0 within the current stage
    #[serde(default)]
    pub progress: f32,
    #[serde(default)]
    pub speed: Option<String>,
    #[serde(default)]
    pub eta: Option<String>,
    #[serde(default)]
    pub output: Option<PathBuf>,
    /// The `<save folder>/<title>.kiripart` package holding in-progress files. Chosen on first start.
    #[serde(default)]
    pub work_dir: Option<PathBuf>,
    /// The downloaded original's final path in the save folder, set as soon as the download finishes.
    /// A restart converts from it instead of downloading again (while it still exists).
    #[serde(default)]
    pub original: Option<PathBuf>,
    /// unix epoch milliseconds
    #[serde(default)]
    pub created_at: u64,
}

impl Job {
    /// Where the output goes: for conversions the chosen folder (else the source's), for downloads the save folder.
    pub fn result_dir(&self, download_dir: &Path) -> PathBuf {
        match &self.source {
            JobSource::Youtube { .. } => download_dir.to_path_buf(),
            JobSource::File { path, output_dir } => output_dir
                .clone()
                .or_else(|| path.parent().map(Path::to_path_buf))
                .unwrap_or_else(|| download_dir.to_path_buf()),
        }
    }
}

/// A job to enqueue. The UI builds it from a probe; for the CLI, Engine::add_url does.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NewJob {
    pub source: JobSource,
    pub title: String,
    pub thumbnail: Option<String>,
    pub duration_secs: Option<f64>,
    pub quality_label: String,
    pub options: JobOptions,
}

/// External executable paths. Tests pass fake script paths.
#[derive(Clone, Debug)]
pub struct Tools {
    pub ytdlp: PathBuf,
    pub deno: PathBuf,
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preset_serde_matches_id() {
        for p in Preset::ALL {
            assert_eq!(serde_json::to_value(p).unwrap(), json!(p.id()));
            assert_eq!(p.id().parse::<Preset>(), Ok(p));
        }
        assert_eq!(Preset::Mp4H264.id(), "mp4-h264");
        assert_eq!(Preset::MovProres.id(), "mov-prores");
        assert!("avi".parse::<Preset>().is_err());
    }

    #[test]
    fn preset_properties() {
        assert_eq!(Preset::Original.extension(), None);
        assert_eq!(Preset::Mp4Hevc.extension(), Some("mp4"));
        assert_eq!(Preset::MovProres.extension(), Some("mov"));
        assert_eq!(Preset::WebmVp9.extension(), Some("webm"));
        assert!(Preset::Mp3.is_audio_only() && Preset::M4a.is_audio_only());
        assert!(!Preset::Mp4H264.is_audio_only());
    }

    #[test]
    fn job_state_json_shape() {
        assert_eq!(
            serde_json::to_value(JobState::Queued).unwrap(),
            json!({"kind": "queued"})
        );
        assert_eq!(
            serde_json::to_value(JobState::Failed("boom".into())).unwrap(),
            json!({"kind": "failed", "message": "boom"})
        );
        let back: JobState = serde_json::from_value(json!({"kind": "encoding"})).unwrap();
        assert_eq!(back, JobState::Encoding);
    }

    #[test]
    fn job_source_json_shape() {
        let y = JobSource::Youtube { url: "u".into() };
        assert_eq!(
            serde_json::to_value(&y).unwrap(),
            json!({"kind": "youtube", "url": "u"})
        );
        let f: JobSource =
            serde_json::from_value(json!({"kind": "file", "path": "/a/b.mkv"})).unwrap();
        assert_eq!(
            f,
            JobSource::File {
                path: "/a/b.mkv".into(),
                output_dir: None
            }
        );
    }

    #[test]
    fn job_state_predicates() {
        assert!(JobState::Downloading.is_active() && JobState::Encoding.is_active());
        assert!(!JobState::Queued.is_active());
        assert!(JobState::Queued.is_pending() && JobState::Encoding.is_pending());
        assert!(!JobState::Stopped.is_pending() && !JobState::Completed.is_pending());
    }
}
