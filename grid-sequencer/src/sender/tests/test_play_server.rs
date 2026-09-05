//! 実サーバーを起こすためのテスト用ハーネス。
//!
//! ここには**判定を書かない**。サーバーの起こし方（ポートと環境変数）と、
//! 死んだときに理由を出すための stderr の溜め方だけを持つ。
//!
//! `realtime-play` 側の `live_ipc/tests/harness.rs` と役割は同じだが、あちらは
//! `#[cfg(test)]` の中にあり crate を跨いで使えない。こちらが要るのは
//! 「起こして繋いで、落とす」だけなので、行の順序を読む道具は持たせていない。

use std::{
    io::{BufRead as _, BufReader},
    net::{SocketAddr, TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use cmrt_runtime::{Config, RealtimeAudioBackend};

/// 実機の play server 実行ファイル。個人のパスをコードへ書かないための入口。
pub(super) const PLAY_SERVER_EXE_ENV: &str = "CMRT_TEST_PLAY_SERVER_EXE";
/// サーバー側が config.toml より優先して読む待ち受けポート。
const PLAY_SERVER_PORT_ENV: &str = "CMRT_REALTIME_PLAY_SERVER_PORT";
const LIVE_INSTANCE_COUNT_ENV: &str = "CMRT_LIVE_INSTANCE_COUNT";
/// サーバー側の**テスト専用**の人工ロード遅延（ミリ秒）。
pub(super) const PATCH_LOAD_DELAY_ENV: &str = "CMRT_TEST_PATCH_LOAD_DELAY_MS";
const SERVER_START_TIMEOUT: Duration = Duration::from_secs(60);

/// テストが自分で起こした play server。**panic しても Drop で必ず落とす**
/// （孤児が残ると共有メモリを掴んだままになり、次の起動を壊す）。
pub(super) struct TestPlayServer {
    child: Child,
    stderr_lines: Arc<Mutex<Vec<String>>>,
}

impl TestPlayServer {
    pub(super) fn spawn(
        exe: &str,
        port: u16,
        live_instance_count: usize,
        extra_env: &[(&str, String)],
    ) -> Self {
        let mut child = Command::new(exe)
            .env(PLAY_SERVER_PORT_ENV, port.to_string())
            .env(LIVE_INSTANCE_COUNT_ENV, live_instance_count.to_string())
            .envs(extra_env.iter().map(|(key, value)| (*key, value.clone())))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("play server を起動できない ({exe}): {error}"));
        let stderr = child.stderr.take().expect("stderr を pipe にしてある");
        let stderr_lines = Arc::new(Mutex::new(Vec::new()));
        let lines = Arc::clone(&stderr_lines);
        // 読み続けないと pipe が詰まってサーバーが止まる。
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                lines.lock().unwrap().push(line);
            }
        });
        let mut server = Self {
            child,
            stderr_lines,
        };
        server.wait_until_listening(port);
        server
    }

    fn wait_until_listening(&mut self, port: u16) {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        let deadline = Instant::now() + SERVER_START_TIMEOUT;
        while Instant::now() < deadline {
            if TcpStream::connect_timeout(&address, Duration::from_millis(150)).is_ok() {
                return;
            }
            // 落ちたら待ち続けない。理由は stderr にしかないのでそれを添えて失敗する。
            if let Some(status) = self.child.try_wait().expect("子プロセスの状態を読めない")
            {
                panic!("play server が {status} で終了した: {}", self.stderr_text());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("play server が待ち受けない: {}", self.stderr_text());
    }

    pub(super) fn stderr_text(&self) -> String {
        self.stderr_lines.lock().unwrap().join(" / ")
    }
}

impl Drop for TestPlayServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// 既に起きているサーバーへ**繋ぐだけ**の設定。
///
/// 実体の指定（注入口）を `exit 0` にしてあるので、supervisor が
/// 自前でサーバーを起こすことはない。ポートを固定できるのが肝で、これが無いと
/// config.toml の既定ポートを使い、起動中の TUI とサーバーを取り合う。
pub(super) fn cfg_for_port(port: u16) -> Config {
    Config {
        realtime_audio_backend: RealtimeAudioBackend::PlayServer,
        realtime_play_server_port: port,
        play_server_launch_override: Some(cmrt_runtime::PlayServerLaunch::ShellCommand(
            "exit 0".to_string(),
        )),
        realtime_play_server_prewarm: false,
        ..Default::default()
    }
}

/// `base` から始めて、**実際に bind できるポート**を1つ選ぶ。
///
/// 固定の `base + pid % 1000` だけだと Windows の予約ポート範囲
/// （`netsh int ipv4 show excludedportrange protocol=tcp`）に当たって、サーバーが
/// `os error 10013` で即死する。当たるかどうかは pid とマシン依存なので、
/// 直った・直っていないの判定に混ざる flaky になる。
pub(super) fn pick_port(base: u16) -> u16 {
    let start = base + (std::process::id() % 1_000) as u16;
    for port in start..start.saturating_add(64) {
        // bind できたら即座に閉じる。listen しただけの socket は TIME_WAIT に
        // 落ちないので、この直後にサーバーが同じポートを取れる。
        if TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).is_ok() {
            return port;
        }
    }
    panic!("{start} から 64 個ぶん探しても bind できるポートが無い");
}
