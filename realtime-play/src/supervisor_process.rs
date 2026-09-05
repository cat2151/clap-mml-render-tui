//! realtime play server プロセスの生存管理。
//!
//! 起動・待ち受け確認・再起動・後始末と、落ちたときの理由の記録をここに集める。
//! HTTP でどう喋るか（[`crate`] 側）とは別の関心事なので、ファイルを分けてある。

use std::{
    net::{SocketAddr, TcpStream},
    process::Child,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context as _, Result};

use crate::{
    logging::{log_realtime_play_event, truncate_for_log},
    process::{self, build_realtime_play_server_command, spawn_realtime_play_server, stop_child},
    server_binary::{resolve_server_binary, ServerBinary},
    startup_failure::{self, ExitLatch, ServerStartupFailure, StderrCapture},
    RealtimePlayServerSupervisor, LIVE_INSTANCE_COUNT_ENV,
};

const PLAY_SERVER_CONNECT_TIMEOUT: Duration = Duration::from_millis(150);
const PLAY_SERVER_START_TIMEOUT: Duration = Duration::from_secs(30);
const PLAY_SERVER_START_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Default)]
pub(crate) struct PlayServerState {
    child: Option<RunningChild>,
    generation: u64,
    /// 同じ理由で spawn し続けないための門番。詳細は [`startup_failure`]。
    exit_latch: ExitLatch,
}

/// 起動中の子プロセスと、それが落ちたときに要る情報。
///
/// 落ちた理由は子の stderr にしかないので、`Child` と同じ寿命で持ち歩く。
struct RunningChild {
    child: Child,
    exe: String,
    stderr_capture: StderrCapture,
}

impl RealtimePlayServerSupervisor {
    pub(crate) fn running_server_generation(&self) -> Result<Option<u64>> {
        let mut state = self.state.lock().unwrap();
        self.drop_exited_child_locked(&mut state)?;
        if self.port_accepts_connections() {
            self.note_server_listening_locked(&mut state);
            return Ok(Some(state.generation));
        }
        if state.child.is_none() {
            return Ok(None);
        }
        self.wait_for_port_locked(&mut state).map(Some)
    }

    pub(crate) fn ensure_started(&self) -> Result<u64> {
        let mut state = self.state.lock().unwrap();
        self.drop_exited_child_locked(&mut state)?;
        if self.port_accepts_connections() {
            self.note_server_listening_locked(&mut state);
            return Ok(state.generation);
        }
        if state.child.is_none() {
            self.spawn_child_locked(&mut state)?;
        }
        self.wait_for_port_locked(&mut state)
    }

    pub fn ensure_started_for_fast_midi(&self) -> Result<()> {
        self.ensure_started().map(|_| ())
    }

    /// このsupervisorが所有する新しいserverだけを起動する。
    ///
    /// catalog計測のようにlive instanceの状態を書き換えるCLIが、TUI等の既存serverへ
    /// 相乗りしないための入口。portが既に使われている場合は何も操作せず失敗する。
    pub fn start_owned_for_fast_midi(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        self.drop_exited_child_locked(&mut state)?;
        if state.child.is_some() {
            anyhow::bail!("realtime play serverはこのsupervisorで既に起動中です");
        }
        if self.port_accepts_connections() {
            anyhow::bail!(
                "realtime play server port {} は既に使用中です。起動中のTUI/serverを終了してから再実行してください",
                self.port
            );
        }
        self.spawn_child_locked(&mut state)?;
        self.wait_for_port_locked(&mut state).map(|_| ())
    }

    pub(crate) fn recover_after_transport_failure(&self, failed_generation: u64) -> Result<u64> {
        let mut state = self.state.lock().unwrap();
        self.drop_exited_child_locked(&mut state)?;

        if state.generation == failed_generation {
            self.restart_locked(&mut state)?;
        } else if state.child.is_none() && !self.port_accepts_connections() {
            self.spawn_child_locked(&mut state)?;
        }

        self.wait_for_port_locked(&mut state)
    }

    fn restart_locked(&self, state: &mut PlayServerState) -> Result<()> {
        stop_child(state.child.take().map(|running| running.child));
        self.bump_generation_locked(state);
        self.spawn_child_locked(state)
    }

    fn spawn_child_locked(&self, state: &mut PlayServerState) -> Result<()> {
        if state.exit_latch.engaged(Instant::now()) {
            return Err(self.latched_startup_error());
        }
        state.child = Some(self.spawn_child()?);
        self.bump_generation_locked(state);
        Ok(())
    }

    fn bump_generation_locked(&self, state: &mut PlayServerState) {
        state.generation = state.generation.wrapping_add(1);
        if state.generation == 0 {
            state.generation = 1;
        }
    }

    fn wait_for_port_locked(&self, state: &mut PlayServerState) -> Result<u64> {
        let deadline = Instant::now() + PLAY_SERVER_START_TIMEOUT;
        loop {
            self.drop_exited_child_locked(state)?;
            if self.port_accepts_connections() {
                self.note_server_listening_locked(state);
                return Ok(state.generation);
            }
            if state.child.is_none() {
                // 打ち切られていれば、ここで理由つきのエラーが返る。落ち続けるサーバーを
                // 30 秒ぶん spawn し直しても、増えるのは死んだプロセスの数だけ。
                self.spawn_child_locked(state)?;
            }
            if Instant::now() >= deadline {
                return Err(self.startup_timeout_error());
            }
            std::thread::sleep(PLAY_SERVER_START_POLL_INTERVAL);
        }
    }

    /// サーバーが待ち受けを始めたときに呼ぶ。門番の数と、直近の失敗理由を捨てる。
    fn note_server_listening_locked(&self, state: &mut PlayServerState) {
        state.exit_latch.reset();
        *self.last_startup_failure.lock().unwrap() = None;
    }

    /// 直近に server が落ちた理由。UI が「無音の理由」を出すために読む。
    pub fn last_startup_failure(&self) -> Option<ServerStartupFailure> {
        self.last_startup_failure.lock().unwrap().clone()
    }

    /// 打ち切りのときのエラー。理由が取れていればそれをそのまま本文にする。
    fn latched_startup_error(&self) -> anyhow::Error {
        match self.last_startup_failure() {
            Some(failure) => anyhow!("{}", failure.message()),
            None => anyhow!(
                "realtime play server が {} 回続けて起動できませんでした",
                startup_failure::MAX_CONSECUTIVE_EXITS
            ),
        }
    }

    /// タイムアウトのときのエラー。「開かなかった」だけでは何も分からないので、
    /// 直近に落ちた理由が分かっていれば添える。
    fn startup_timeout_error(&self) -> anyhow::Error {
        let timeout = format!(
            "realtime play server did not start listening on 127.0.0.1:{} within {:?}",
            self.port, PLAY_SERVER_START_TIMEOUT
        );
        match self.last_startup_failure() {
            Some(failure) => anyhow!("{timeout} / {}", failure.message()),
            None => anyhow!(timeout),
        }
    }

    fn drop_exited_child_locked(&self, state: &mut PlayServerState) -> Result<()> {
        let Some(running) = state.child.as_mut() else {
            return Ok(());
        };
        let Some(status) = running
            .child
            .try_wait()
            .with_context(|| "realtime play server child status check failed")?
        else {
            return Ok(());
        };
        // 落ちた理由は子の stderr にしかない。child を捨てる前にここで拾う。
        let stderr_tail = running.stderr_capture.drain_snapshot();
        log_realtime_play_event(format!(
            "action=server-exited exit={} exe={:?} stderr_tail={:?}",
            status
                .code()
                .map_or_else(|| "不明".to_owned(), |code| code.to_string()),
            running.exe,
            truncate_for_log(&stderr_tail.join(" / "), 1_000)
        ));
        let failure = ServerStartupFailure::Exited {
            exe: running.exe.clone(),
            exit_code: status.code(),
            stderr_tail,
        };
        *self.last_startup_failure.lock().unwrap() = Some(failure);
        state.child = None;
        state.exit_latch.record_exit(Instant::now());
        self.bump_generation_locked(state);
        Ok(())
    }

    fn port_accepts_connections(&self) -> bool {
        TcpStream::connect_timeout(&self.socket_addr(), PLAY_SERVER_CONNECT_TIMEOUT).is_ok()
    }

    fn socket_addr(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.port))
    }

    fn spawn_child(&self) -> Result<RunningChild> {
        let launch = self.build_command()?;
        let stderr_capture = StderrCapture::default();
        let child = spawn_realtime_play_server(
            launch.command,
            &launch.description,
            self.port,
            self.live_instance_count,
            Arc::clone(&self.startup_progress),
            stderr_capture.clone(),
        )?;
        Ok(RunningChild {
            child,
            exe: launch.resolved.exe,
            stderr_capture,
        })
    }

    /// 掴む実体。**起動時に 1 度決めて持ち回る。**
    ///
    /// 解決は起動のたびに走るのではなく、ここで 1 度だけ。supervisor がサーバーを
    /// 起こし直しても同じ実体が選ばれる（途中で入れ替わると、画面に出した profile が嘘になる）。
    pub fn server_binary(&self) -> &ServerBinary {
        self.server_binary.get_or_init(|| {
            let binary = resolve_server_binary(self.launch_override.as_ref());
            match &binary {
                ServerBinary::Resolved(resolved) => log_realtime_play_event(format!(
                    "action=server-resolved {}",
                    resolved.log_fields()
                )),
                ServerBinary::NotFound { searched } => log_realtime_play_event(format!(
                    "action=server-not-found searched={:?}",
                    searched.join(" / ")
                )),
            }
            binary
        })
    }

    fn build_command(&self) -> Result<process::ServerLaunch> {
        let resolved = match self.server_binary() {
            ServerBinary::Resolved(resolved) => resolved,
            ServerBinary::NotFound { searched } => {
                // 素の実行ファイル名で spawn して OS のエラーに任せると、
                // 「どこを探したのか」が誰にも分からなくなる。
                let failure = ServerStartupFailure::NotFound {
                    searched: searched.clone(),
                };
                let message = failure.message();
                *self.last_startup_failure.lock().unwrap() = Some(failure);
                return Err(anyhow!(message));
            }
        };
        let mut launch = build_realtime_play_server_command(resolved);
        launch.command.env(
            LIVE_INSTANCE_COUNT_ENV,
            self.live_instance_count.to_string(),
        );
        Ok(launch)
    }
}

impl Drop for RealtimePlayServerSupervisor {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            stop_child(state.child.take().map(|running| running.child));
        }
    }
}

#[cfg(test)]
mod tests;
