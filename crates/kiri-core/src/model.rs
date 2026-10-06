//! 큐와 CLI 가 함께 쓰는 작업 모델. serde 표현이 곧 프론트엔드·CLI 계약이다.
use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf, str::FromStr};

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

    /// 재인코딩 결과 확장자. 원본 유지는 인코딩하지 않으므로 None.
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JobOptions {
    /// 비디오 format_id. None 이면 오디오만 받는다.
    pub format_id: Option<String>,
    pub preset: Preset,
    pub subtitles: Vec<String>,
    /// subtitles 중 자동 생성 자막이 섞여 있으면 true (--write-auto-subs).
    pub auto_subtitles: bool,
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
    /// 프로세스가 돌고 있는 상태.
    pub fn is_active(&self) -> bool {
        matches!(self, JobState::Downloading | JobState::Encoding)
    }
    /// 아직 끝나지 않은 상태(대기 포함).
    pub fn is_pending(&self) -> bool {
        matches!(self, JobState::Queued) || self.is_active()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Job {
    pub id: u64,
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub duration_secs: Option<f64>,
    /// 화면 표시용 화질 ("1080p60", 오디오만이면 "audio").
    #[serde(default)]
    pub quality_label: String,
    pub options: JobOptions,
    pub state: JobState,
    /// 현재 단계 기준 0.0~1.0
    #[serde(default)]
    pub progress: f32,
    #[serde(default)]
    pub speed: Option<String>,
    #[serde(default)]
    pub eta: Option<String>,
    #[serde(default)]
    pub output: Option<PathBuf>,
    /// unix epoch milliseconds
    #[serde(default)]
    pub created_at: u64,
}

/// 큐에 넣을 작업. UI 는 probe 결과로, CLI 는 Engine::add_url 이 만든다.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NewJob {
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub duration_secs: Option<f64>,
    pub quality_label: String,
    pub options: JobOptions,
}

/// 외부 실행 파일 위치. 테스트는 가짜 스크립트 경로를 넣는다.
#[derive(Clone, Debug)]
pub struct Tools {
    pub ytdlp: PathBuf,
    pub deno: PathBuf,
    pub ffmpeg: PathBuf,
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
    fn job_state_predicates() {
        assert!(JobState::Downloading.is_active() && JobState::Encoding.is_active());
        assert!(!JobState::Queued.is_active());
        assert!(JobState::Queued.is_pending() && JobState::Encoding.is_pending());
        assert!(!JobState::Stopped.is_pending() && !JobState::Completed.is_pending());
    }
}
