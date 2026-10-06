//! kiri CLI: 실행 중인 kiri 앱의 큐를 조회·제어한다.
use clap::{Parser, Subcommand};
use kiri_core::{
    ipc::{self, ClientError, Request, Response},
    model::{Job, JobState},
};
use std::{path::Path, process::ExitCode, time::Duration};

const BUNDLE_ID: &str = "org.bobpark.kiri";

#[derive(Parser, Debug)]
#[command(name = "kiri", version, about = "Control the kiri YouTube downloader")]
struct Cli {
    /// Print the raw JSON response
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Jobs currently downloading or encoding
    Status,
    /// All jobs in the queue
    List,
    /// Add a YouTube URL to the queue
    Add {
        url: String,
        /// best | audio | 1080p | 720p | …
        #[arg(long)]
        quality: Option<String>,
        /// original | mp4-h264 | mp4-hevc | mov-prores | webm-vp9 | mp3 | m4a
        #[arg(long)]
        format: Option<String>,
        /// Subtitle languages, comma separated (ko,en)
        #[arg(long, value_delimiter = ',')]
        subs: Option<Vec<String>>,
    },
    /// Remove a job (stops it first if running)
    Remove { id: u64 },
    /// Stop a running or queued job
    Stop { id: u64 },
}

impl Cmd {
    fn into_request(self) -> Request {
        match self {
            Cmd::Status => Request::Status,
            Cmd::List => Request::List,
            Cmd::Add {
                url,
                quality,
                format,
                subs,
            } => Request::Add {
                url,
                quality,
                preset: format,
                subs,
            },
            Cmd::Remove { id } => Request::Remove { id },
            Cmd::Stop { id } => Request::Stop { id },
        }
    }
}

fn state_name(s: &JobState) -> &'static str {
    match s {
        JobState::Queued => "queued",
        JobState::Downloading => "downloading",
        JobState::Encoding => "encoding",
        JobState::Completed => "completed",
        JobState::Failed(_) => "failed",
        JobState::Stopped => "stopped",
    }
}

fn format_jobs(jobs: &[Job]) -> String {
    if jobs.is_empty() {
        return "no jobs".into();
    }
    let rows: Vec<[String; 6]> = jobs
        .iter()
        .map(|j| {
            let title = match &j.state {
                JobState::Failed(m) => format!("{} ({m})", j.title),
                _ => j.title.clone(),
            };
            [
                j.id.to_string(),
                state_name(&j.state).into(),
                format!("{}%", (j.progress * 100.0).round() as u32),
                j.speed.clone().unwrap_or_else(|| "-".into()),
                j.eta.clone().unwrap_or_else(|| "-".into()),
                title,
            ]
        })
        .collect();
    let header = ["ID", "STATE", "PROGRESS", "SPEED", "ETA", "TITLE"].map(String::from);
    let widths: Vec<usize> = (0..5)
        .map(|c| {
            rows.iter()
                .map(|r| r[c].len())
                .chain([header[c].len()])
                .max()
                .unwrap()
        })
        .collect();
    let line = |r: &[String; 6]| {
        let mut s = String::new();
        for c in 0..5 {
            s.push_str(&format!("{:<w$}  ", r[c], w = widths[c]));
        }
        s.push_str(&r[5]);
        s
    };
    std::iter::once(line(&header))
        .chain(rows.iter().map(line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 응답 출력 후 종료 코드.
fn print_response(resp: &Response, json: bool) -> ExitCode {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(resp).expect("response serializes")
        );
    } else {
        match resp {
            Response::Jobs { jobs } => println!("{}", format_jobs(jobs)),
            Response::Added { job } => println!("added #{}: {}", job.id, job.title),
            Response::Ok => println!("ok"),
            Response::Error { message, .. } => eprintln!("error: {message}"),
        }
    }
    match resp {
        Response::Error { .. } => ExitCode::from(1),
        _ => ExitCode::SUCCESS,
    }
}

async fn launch_and_wait(socket: &Path) -> bool {
    let launched = std::process::Command::new("open")
        .args(["-b", BUNDLE_ID])
        .status()
        .is_ok_and(|s| s.success());
    if !launched {
        return false;
    }
    for _ in 0..50 {
        if tokio::net::UnixStream::connect(socket).await.is_ok() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    false
}

async fn run(cli: Cli) -> ExitCode {
    let socket = ipc::socket_path();
    let is_add = matches!(cli.cmd, Cmd::Add { .. });
    let req = cli.cmd.into_request();
    let mut result = ipc::request(&socket, &req).await;
    if is_add && result == Err(ClientError::NotRunning) && launch_and_wait(&socket).await {
        result = ipc::request(&socket, &req).await;
    }
    match result {
        Ok(resp) => print_response(&resp, cli.json),
        Err(ClientError::NotRunning) => {
            eprintln!("kiri app is not running");
            ExitCode::from(2)
        }
        Err(ClientError::Io(e)) => {
            eprintln!("cannot talk to kiri app: {e}");
            ExitCode::from(2)
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(run(cli))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiri_core::model::{JobOptions, Preset};

    fn job(id: u64, state: JobState, progress: f32) -> Job {
        Job {
            id,
            url: "u".into(),
            title: "Rust in 100 Seconds".into(),
            thumbnail: None,
            duration_secs: None,
            quality_label: "1080p60".into(),
            options: JobOptions {
                format_id: None,
                preset: Preset::Original,
                subtitles: vec![],
                auto_subtitles: false,
            },
            state,
            progress,
            speed: Some("2.1 MB/s".into()),
            eta: Some("00:12".into()),
            output: None,
            created_at: 0,
        }
    }

    #[test]
    fn formats_table() {
        let out = format_jobs(&[
            job(1, JobState::Downloading, 0.48),
            job(12, JobState::Failed("boom".into()), 0.0),
        ]);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(
            lines[0],
            "ID  STATE        PROGRESS  SPEED     ETA    TITLE"
        );
        assert_eq!(
            lines[1],
            "1   downloading  48%       2.1 MB/s  00:12  Rust in 100 Seconds"
        );
        assert_eq!(
            lines[2],
            "12  failed       0%        2.1 MB/s  00:12  Rust in 100 Seconds (boom)"
        );
    }

    #[test]
    fn empty_table_message() {
        assert_eq!(format_jobs(&[]), "no jobs");
    }

    #[test]
    fn parses_add_args() {
        let cli = Cli::try_parse_from([
            "kiri",
            "add",
            "https://youtu.be/x",
            "--quality",
            "720p",
            "--format",
            "mp3",
            "--subs",
            "ko,en",
            "--json",
        ])
        .unwrap();
        assert!(cli.json);
        assert_eq!(
            cli.cmd.into_request(),
            Request::Add {
                url: "https://youtu.be/x".into(),
                quality: Some("720p".into()),
                preset: Some("mp3".into()),
                subs: Some(vec!["ko".into(), "en".into()]),
            }
        );
    }
}
