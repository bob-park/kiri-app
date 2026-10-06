//! 자식 프로세스 실행. 줄 단위 출력 콜백, 취소(프로세스 그룹 kill), 실패 시 stderr 꼬리.
use std::{collections::VecDeque, fmt, path::Path, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;

const TAIL: usize = 20;

#[derive(Debug, Clone, PartialEq)]
pub enum RunError {
    Cancelled,
    Spawn(String),
    Failed { tail: Vec<String> },
}

impl RunError {
    /// 사용자에게 보여줄 한 줄: stderr 의 마지막 비어 있지 않은 줄.
    pub fn summary(&self) -> String {
        match self {
            RunError::Cancelled => "cancelled".into(),
            RunError::Spawn(e) => e.clone(),
            RunError::Failed { tail } => tail
                .iter()
                .rev()
                .find(|l| !l.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| "process failed".into()),
        }
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.summary())
    }
}

fn command(program: &Path, args: &[String]) -> Command {
    let mut c = Command::new(program);
    c.args(args)
        .stdin(Stdio::null())
        .process_group(0) // 취소 시 손자까지 한 번에 죽이기 위해
        .kill_on_drop(true);
    c
}

fn kill_group(child: &tokio::process::Child) {
    if let Some(pid) = child.id() {
        // SAFETY: killpg 는 시그널만 보낸다. 실패(이미 종료)는 무시한다.
        unsafe {
            libc::killpg(pid as i32, libc::SIGKILL);
        }
    }
}

pub async fn run(
    program: &Path,
    args: &[String],
    cancel: &CancellationToken,
    mut on_line: impl FnMut(&str),
) -> Result<(), RunError> {
    let mut child = command(program, args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| RunError::Spawn(format!("{}: {e}", program.display())))?;

    let (tx, mut rx) = mpsc::unbounded_channel::<(bool, String)>();
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let tx_err = tx.clone();
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            if tx.send((false, l)).is_err() {
                break;
            }
        }
    });
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            if tx_err.send((true, l)).is_err() {
                break;
            }
        }
    });

    let mut tail: VecDeque<String> = VecDeque::with_capacity(TAIL);
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                kill_group(&child);
                let _ = child.wait().await;
                return Err(RunError::Cancelled);
            }
            msg = rx.recv() => match msg {
                Some((is_err, line)) => {
                    if is_err {
                        if tail.len() == TAIL {
                            tail.pop_front();
                        }
                        tail.push_back(line.clone());
                    }
                    on_line(&line);
                }
                None => break, // 두 스트림 모두 닫힘
            }
        }
    }
    let status = tokio::select! {
        _ = cancel.cancelled() => {
            kill_group(&child);
            let _ = child.wait().await;
            return Err(RunError::Cancelled);
        }
        s = child.wait() => s.map_err(|e| RunError::Spawn(e.to_string()))?,
    };
    if status.success() {
        Ok(())
    } else {
        Err(RunError::Failed {
            tail: Vec::from(tail),
        })
    }
}

/// 짧은 명령의 stdout 전체 (probe, --version).
pub async fn output(program: &Path, args: &[String]) -> Result<String, RunError> {
    let out = command(program, args)
        .output()
        .await
        .map_err(|e| RunError::Spawn(format!("{}: {e}", program.display())))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    let lines: Vec<&str> = err.lines().collect();
    let start = lines.len().saturating_sub(TAIL);
    Err(RunError::Failed {
        tail: lines[start..].iter().map(|s| s.to_string()).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::script;
    use std::time::Duration;

    #[tokio::test]
    async fn collects_stdout_and_stderr_lines() {
        let d = tempfile::tempdir().unwrap();
        let s = script(d.path(), "ok", "echo out1\necho err1 >&2");
        let mut lines = vec![];
        run(&s, &[], &CancellationToken::new(), |l| {
            lines.push(l.to_string())
        })
        .await
        .unwrap();
        lines.sort();
        assert_eq!(lines, vec!["err1", "out1"]);
    }

    #[tokio::test]
    async fn failure_returns_stderr_tail() {
        let d = tempfile::tempdir().unwrap();
        let s = script(
            d.path(),
            "fail",
            "echo noise\necho 'ERROR: Video unavailable' >&2\nexit 3",
        );
        let err = run(&s, &[], &CancellationToken::new(), |_| {})
            .await
            .unwrap_err();
        assert_eq!(
            err,
            RunError::Failed {
                tail: vec!["ERROR: Video unavailable".into()]
            }
        );
        assert_eq!(err.summary(), "ERROR: Video unavailable");
    }

    #[tokio::test]
    async fn spawn_error_for_missing_binary() {
        let err = run(
            Path::new("/nonexistent/yt-dlp"),
            &[],
            &CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap_err();
        assert!(matches!(err, RunError::Spawn(_)), "{err:?}");
    }

    #[tokio::test]
    async fn cancel_kills_whole_process_group() {
        let d = tempfile::tempdir().unwrap();
        // 손자 프로세스(sleep)의 pid 를 찍고 기다린다.
        let s = script(d.path(), "slow", "sleep 30 &\necho $!\nwait");
        let token = CancellationToken::new();
        let t2 = token.clone();
        let (pid_tx, pid_rx) = std::sync::mpsc::channel::<i32>();
        let task = tokio::spawn(async move {
            run(&s, &[], &t2, |l| {
                if let Ok(pid) = l.trim().parse() {
                    let _ = pid_tx.send(pid);
                }
            })
            .await
        });
        let grandchild = tokio::task::spawn_blocking(move || {
            pid_rx.recv_timeout(Duration::from_secs(5)).unwrap()
        })
        .await
        .unwrap();
        token.cancel();
        let res = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(res, Err(RunError::Cancelled));
        tokio::time::sleep(Duration::from_millis(200)).await;
        let alive = unsafe { libc::kill(grandchild, 0) } == 0;
        assert!(!alive, "grandchild {grandchild} survived cancel");
    }

    #[tokio::test]
    async fn output_returns_stdout() {
        let d = tempfile::tempdir().unwrap();
        let s = script(d.path(), "v", "echo 2026.09.30");
        assert_eq!(output(&s, &[]).await.unwrap(), "2026.09.30\n");
        let f = script(d.path(), "f", "echo bad >&2; exit 1");
        assert_eq!(output(&f, &[]).await.unwrap_err().summary(), "bad");
    }
}
