//! CLI ↔ 앱. 연결당 요청 한 줄, 응답 한 줄 (JSON + \n).
use crate::{
    engine::{Engine, EngineError},
    model::Job,
};
use serde::{Deserialize, Serialize};
use std::{
    io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Envelope<T> {
    pub v: u32,
    pub body: T,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Status,
    List,
    Add {
        url: String,
        #[serde(default)]
        quality: Option<String>,
        #[serde(default)]
        preset: Option<String>,
        #[serde(default)]
        subs: Option<Vec<String>>,
    },
    Remove {
        id: u64,
    },
    Stop {
        id: u64,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Jobs { jobs: Vec<Job> },
    Added { job: Job },
    Ok,
    Error { code: String, message: String },
}

impl From<EngineError> for Response {
    fn from(e: EngineError) -> Self {
        Response::Error {
            code: e.code().into(),
            message: e.to_string(),
        }
    }
}

pub fn socket_path() -> PathBuf {
    if let Some(p) = std::env::var_os("KIRI_SOCKET") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join("Library/Application Support/org.bobpark.kiri/kiri.sock")
}

/// 다른 인스턴스가 응답하면 AddrInUse. 응답 없는 파일은 지우고 다시 바인드한다.
/// ponytail: connect 성공을 생존 신호로 쓴다. 죽은 직후의 소켓 fd 사본을 fork 중인
/// 자식이 exec 전까지 들고 있는 찰나에는 살아 있어 보인다 — 다음 실행에서 풀린다.
pub async fn bind(path: &Path) -> io::Result<UnixListener> {
    if UnixStream::connect(path).await.is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "another kiri instance owns the socket",
        ));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

pub async fn serve(listener: UnixListener, engine: Engine) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let engine = engine.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_conn(stream, engine).await {
                        eprintln!("kiri ipc: {e}");
                    }
                });
            }
            Err(e) => eprintln!("kiri ipc: accept failed: {e}"),
        }
    }
}

async fn handle_conn(stream: UnixStream, engine: Engine) -> io::Result<()> {
    let (r, mut w) = stream.into_split();
    let mut line = String::new();
    BufReader::new(r).read_line(&mut line).await?;
    let bad = |m: String| Response::Error {
        code: "bad_request".into(),
        message: m,
    };
    let resp = match serde_json::from_str::<Envelope<serde_json::Value>>(&line) {
        Err(e) => bad(e.to_string()),
        Ok(env) if env.v != PROTOCOL_VERSION => Response::Error {
            code: "version_mismatch".into(),
            message: format!(
                "app speaks protocol v{PROTOCOL_VERSION}, client sent v{}",
                env.v
            ),
        },
        Ok(env) => match serde_json::from_value::<Request>(env.body) {
            Ok(req) => dispatch(&engine, req).await,
            Err(e) => bad(e.to_string()),
        },
    };
    let mut out = serde_json::to_string(&Envelope {
        v: PROTOCOL_VERSION,
        body: resp,
    })
    .map_err(io::Error::other)?;
    out.push('\n');
    w.write_all(out.as_bytes()).await
}

pub async fn dispatch(engine: &Engine, req: Request) -> Response {
    let done = |r: Result<(), EngineError>| r.map_or_else(Response::from, |_| Response::Ok);
    match req {
        Request::Status => Response::Jobs {
            jobs: engine.running(),
        },
        Request::List => Response::Jobs {
            jobs: engine.list(),
        },
        Request::Add {
            url,
            quality,
            preset,
            subs,
        } => match engine.add_url(&url, quality, preset, subs).await {
            Ok(job) => Response::Added { job },
            Err(e) => e.into(),
        },
        Request::Remove { id } => done(engine.remove(id)),
        Request::Stop { id } => done(engine.stop(id)),
    }
}

#[derive(Debug, PartialEq)]
pub enum ClientError {
    NotRunning,
    Io(String),
}

pub async fn request(path: &Path, req: &Request) -> Result<Response, ClientError> {
    let stream = UnixStream::connect(path)
        .await
        .map_err(|e| match e.kind() {
            io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => ClientError::NotRunning,
            _ => ClientError::Io(e.to_string()),
        })?;
    let io_err = |e: io::Error| ClientError::Io(e.to_string());
    let (r, mut w) = stream.into_split();
    let mut out = serde_json::to_string(&Envelope {
        v: PROTOCOL_VERSION,
        body: req,
    })
    .map_err(|e| ClientError::Io(e.to_string()))?;
    out.push('\n');
    w.write_all(out.as_bytes()).await.map_err(io_err)?;
    let mut line = String::new();
    BufReader::new(r)
        .read_line(&mut line)
        .await
        .map_err(io_err)?;
    let env: Envelope<Response> =
        serde_json::from_str(&line).map_err(|e| ClientError::Io(e.to_string()))?;
    Ok(env.body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        engine::{EngineConfig, EnginePaths},
        model::{Preset, Tools},
    };
    use serde_json::json;
    use std::os::unix::fs::FileTypeExt;

    fn idle_engine(dir: &Path) -> Engine {
        let tools = Tools {
            ytdlp: dir.join("none/yt-dlp"),
            deno: dir.join("none/deno"),
            ffmpeg: dir.join("none/ffmpeg"),
            ffprobe: dir.join("none/ffprobe"),
        };
        let paths = EnginePaths {
            queue_file: dir.join("q.json"),
            cache_dir: dir.join("c"),
            log_dir: dir.join("l"),
        };
        let cfg = EngineConfig {
            max_concurrent: 1,
            hw_accel: false,
            download_dir: dir.join("out"),
            default_quality: "best".into(),
            default_preset: Preset::Original,
            default_subtitles: vec![],
        };
        Engine::new(paths, tools, cfg, tokio::runtime::Handle::current(), |_| {})
    }

    async fn server(dir: &Path) -> PathBuf {
        let path = dir.join("kiri.sock");
        let listener = bind(&path).await.unwrap();
        tokio::spawn(serve(listener, idle_engine(dir)));
        path
    }

    #[test]
    fn request_json_shape() {
        let env = Envelope {
            v: 1,
            body: Request::Stop { id: 3 },
        };
        assert_eq!(
            serde_json::to_value(&env).unwrap(),
            json!({"v": 1, "body": {"type": "stop", "id": 3}})
        );
        let add: Request = serde_json::from_value(json!({"type": "add", "url": "u"})).unwrap();
        assert_eq!(
            add,
            Request::Add {
                url: "u".into(),
                quality: None,
                preset: None,
                subs: None
            }
        );
        assert_eq!(
            serde_json::to_value(Response::Ok).unwrap(),
            json!({"type": "ok"})
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn list_and_errors_over_socket() {
        let d = tempfile::tempdir().unwrap();
        let path = server(d.path()).await;
        assert_eq!(
            request(&path, &Request::List).await.unwrap(),
            Response::Jobs { jobs: vec![] }
        );
        assert_eq!(
            request(&path, &Request::Status).await.unwrap(),
            Response::Jobs { jobs: vec![] }
        );
        let Response::Error { code, .. } = request(&path, &Request::Stop { id: 7 }).await.unwrap()
        else {
            panic!()
        };
        assert_eq!(code, "not_found");
        let r = request(
            &path,
            &Request::Add {
                url: "hello".into(),
                quality: None,
                preset: None,
                subs: None,
            },
        )
        .await
        .unwrap();
        assert!(
            matches!(r, Response::Error { ref code, .. } if code == "invalid_url"),
            "{r:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn version_mismatch_is_reported() {
        let d = tempfile::tempdir().unwrap();
        let path = server(d.path()).await;
        let mut s = UnixStream::connect(&path).await.unwrap();
        s.write_all(b"{\"v\":99,\"body\":{\"type\":\"list\"}}\n")
            .await
            .unwrap();
        let mut line = String::new();
        BufReader::new(s).read_line(&mut line).await.unwrap();
        let env: Envelope<Response> = serde_json::from_str(&line).unwrap();
        assert!(matches!(env.body, Response::Error { ref code, .. } if code == "version_mismatch"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bind_sets_0600_and_refuses_live_server() {
        let d = tempfile::tempdir().unwrap();
        let path = server(d.path()).await;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(
            bind(&path).await.unwrap_err().kind(),
            io::ErrorKind::AddrInUse
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bind_replaces_stale_socket_file() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("kiri.sock");
        // 여기서 리스너를 bind 했다 drop 하면 안 된다: 같은 프로세스의 다른 테스트가
        // 그 사이 fork 하면 자식이 fd 사본을 들고(CLOEXEC 은 exec 때 닫는다) 닫은
        // 소켓을 잠시 살려 둬, bind 의 connect 탐색이 "다른 인스턴스가 있다"로 본다.
        // 전제는 "서버 없는 파일"이므로 파일만 만든다.
        std::fs::write(&path, b"").unwrap();
        bind(&path).await.unwrap();
        assert!(std::fs::metadata(&path).unwrap().file_type().is_socket());
    }

    #[tokio::test]
    async fn client_reports_not_running() {
        let d = tempfile::tempdir().unwrap();
        let r = request(&d.path().join("none.sock"), &Request::List).await;
        assert_eq!(r, Err(ClientError::NotRunning));
    }

    #[test]
    fn socket_path_env_override() {
        // 다른 테스트와 env 를 공유하므로 이 테스트 안에서만 설정·해제한다.
        unsafe { std::env::set_var("KIRI_SOCKET", "/tmp/x.sock") };
        assert_eq!(socket_path(), PathBuf::from("/tmp/x.sock"));
        unsafe { std::env::remove_var("KIRI_SOCKET") };
        assert!(socket_path().ends_with("Library/Application Support/org.bobpark.kiri/kiri.sock"));
    }
}
