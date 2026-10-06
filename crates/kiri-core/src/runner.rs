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

/// 프로세스 그룹을 지키는 가드. `kill_on_drop` 은 직접 자식만 죽이므로, future 가
/// drop 되면(런타임 종료, task abort, timeout) 손자가 그룹에 남는다. Drop 에서
/// 그룹 전체를 죽이고, 정상적으로 wait 해 reap 한 뒤에는 disarm 한다 — 재활용된
/// pgid 에 시그널을 보내지 않기 위해.
struct GroupGuard(Option<i32>);

impl GroupGuard {
    fn arm(child: &tokio::process::Child) -> Self {
        // process_group(0) 로 띄웠으므로 자식 pid == pgid.
        Self(child.id().map(|p| p as i32))
    }

    fn kill(&self) {
        if let Some(pgid) = self.0 {
            // SAFETY: killpg 는 시그널만 보낸다. 실패(이미 종료)는 무시한다.
            unsafe {
                libc::killpg(pgid, libc::SIGKILL);
            }
        }
    }

    fn disarm(&mut self) {
        self.0 = None;
    }
}

impl Drop for GroupGuard {
    fn drop(&mut self) {
        self.kill();
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
    // child 보다 늦게 선언 → 더 먼저 drop 된다(자식이 아직 살아 있는 동안 killpg).
    let mut guard = GroupGuard::arm(&child);

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
                guard.kill();
                let _ = child.start_kill(); // killpg 가 못 닿는 경우의 대비
                let _ = child.wait().await;
                guard.disarm();
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
            guard.kill();
            let _ = child.start_kill(); // killpg 가 못 닿는 경우의 대비
            let _ = child.wait().await;
            guard.disarm();
            return Err(RunError::Cancelled);
        }
        s = child.wait() => s.map_err(|e| RunError::Spawn(e.to_string()))?,
    };
    guard.disarm(); // reap 완료
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
    let child = command(program, args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| RunError::Spawn(format!("{}: {e}", program.display())))?;
    let mut guard = GroupGuard::arm(&child);
    let out = child
        .wait_with_output()
        .await
        .map_err(|e| RunError::Spawn(format!("{}: {e}", program.display())))?;
    guard.disarm(); // reap 완료
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

    /// 손자(`sleep 30`)를 띄우는 스크립트를 task 로 돌리고 그 손자 pid 를 준다.
    async fn spawn_slow(
        dir: &Path,
        cancel: CancellationToken,
    ) -> (tokio::task::JoinHandle<Result<(), RunError>>, i32) {
        let s = script(dir, "slow", "sleep 30 &\necho $!\nwait");
        let (pid_tx, pid_rx) = std::sync::mpsc::channel::<i32>();
        let task = tokio::spawn(async move {
            run(&s, &[], &cancel, |l| {
                if let Ok(pid) = l.trim().parse() {
                    let _ = pid_tx.send(pid);
                }
            })
            .await
        });
        let pid = tokio::task::spawn_blocking(move || {
            pid_rx.recv_timeout(Duration::from_secs(5)).unwrap()
        })
        .await
        .unwrap();
        (task, pid)
    }

    /// pid 가 사라질 때까지 최대 2초 폴링.
    async fn gone(pid: i32) -> bool {
        for _ in 0..100 {
            if unsafe { libc::kill(pid, 0) } != 0 {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        false
    }

    #[tokio::test]
    async fn cancel_kills_whole_process_group() {
        let d = tempfile::tempdir().unwrap();
        let token = CancellationToken::new();
        let (task, grandchild) = spawn_slow(d.path(), token.clone()).await;
        token.cancel();
        let res = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(res, Err(RunError::Cancelled));
        assert!(
            gone(grandchild).await,
            "grandchild {grandchild} survived cancel"
        );
    }

    /// 취소 토큰 없이 future 가 drop 되는 경로(런타임 종료, task abort, timeout).
    #[tokio::test]
    async fn dropping_the_future_kills_whole_process_group() {
        let d = tempfile::tempdir().unwrap();
        let (task, grandchild) = spawn_slow(d.path(), CancellationToken::new()).await;
        task.abort();
        assert!(
            gone(grandchild).await,
            "grandchild {grandchild} survived drop"
        );
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
