//! 実サーバーを起こして stderr で判定するためのテスト用ハーネス。
//!
//! ここには**判定そのものは書かない**。サーバーの起こし方（ポートと環境変数）と、
//! 出てきた行の読み方だけを持つ。

use std::{
    io::{BufRead as _, BufReader},
    net::{SocketAddr, TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// 実機の play server 実行ファイル。個人のパスをコードへ書かないための入口。
pub(super) const PLAY_SERVER_EXE_ENV: &str = "CMRT_TEST_PLAY_SERVER_EXE";
/// サーバー側が config.toml より優先して読む待ち受けポート。
pub(super) const PLAY_SERVER_PORT_ENV: &str = "CMRT_REALTIME_PLAY_SERVER_PORT";
pub(super) const LIVE_INSTANCE_COUNT_ENV: &str = "CMRT_LIVE_INSTANCE_COUNT";
/// サーバー側の**テスト専用**の人工ロード遅延（ミリ秒）。
/// 実体は play-server の `player/worker/bank/state.rs`。
pub(super) const PATCH_LOAD_DELAY_ENV: &str = "CMRT_TEST_PATCH_LOAD_DELAY_MS";
/// 人工ロードで止める長さ。受け入れ条件 2 の「500ms の人工 patch load」。
pub(super) const SLOW_PATCH_LOAD_MS: u64 = 500;
/// サーバー全体で持つ予備インスタンスの数。
pub(super) const SPARE_INSTANCES_ENV: &str = "CMRT_SPARE_INSTANCES";
/// **既定プラグイン以外の音色**へ先読みするときの音色パス。
///
/// 何を渡すかはマシンのプラグイン導入状況で変わる（Dexed の cartridge、
/// Vaporizer2 の `.vvp` など）ので、コードへ書かずここで受ける。
/// 渡さなければ、それを要求するテストは skip される。
pub(super) const STANDBY_PATCH_ENV: &str = "CMRT_TEST_STANDBY_PATCH";
pub(super) const SERVER_START_TIMEOUT: Duration = Duration::from_secs(60);

/// `base` から始めて、**実際に bind できるポート**を1つ選ぶ。
///
/// 固定の `base + pid % 1000` だけだと Windows の予約ポート範囲
/// （`netsh int ipv4 show excludedportrange protocol=tcp`。この開発機では
/// 49745-49944 / 50000-50059 など）に当たって、サーバーが
/// `os error 10013` で即死する。当たるかどうかは pid とマシン依存なので、
/// 直った・直っていないの判定に混ざる flaky になる。**空いている所を探す**のが
/// 唯一の安定した手当て。
///
/// 帯は test ごとに分けてあるので、探索幅を狭く保てば他の test と重ならない。
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

/// サーバーのログ行から `thread=ThreadId(..)` を取り出す。
pub(super) fn thread_id_of(line: &str) -> String {
    let rest = line
        .split_once("thread=")
        .unwrap_or_else(|| panic!("thread= を含まない行: {line}"))
        .1;
    rest.split_whitespace()
        .next()
        .expect("thread= の値が空")
        .to_string()
}

/// テスト用に起こした play server。**panic しても Drop で必ず落とす**
/// （孤児が残ると共有メモリを掴んだままになり、次の起動を壊す）。
pub(super) struct TestPlayServer {
    child: Child,
    stderr_lines: Arc<Mutex<Vec<String>>>,
}

impl TestPlayServer {
    pub(super) fn spawn(exe: &str, port: u16, live_instance_count: usize) -> Self {
        Self::spawn_with_env(exe, port, live_instance_count, &[])
    }

    /// 追加の環境変数つきで起こす。**渡すのは子プロセスの環境だけ**なので、
    /// 同時に走る他のテストにも、このプロセス自身にも影響しない。
    pub(super) fn spawn_with_env(
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
        // 読み続けないと pipe が詰まってサーバーが止まる。行はそのまま溜める。
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
            // 落ちたら待ち続けない。理由は stderr にしか無いのでそれを添えて即座に失敗する。
            if let Some(status) = self.child.try_wait().expect("子プロセスの状態を読めない")
            {
                panic!("play server が {status} で終了した: {}", self.stderr_text());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("play server が待ち受けない: {}", self.stderr_text());
    }

    /// 条件に合う stderr 行が出るまで待って、その行を返す。
    pub(super) fn wait_for_stderr_line(&self, matches: impl Fn(&str) -> bool) -> String {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(line) = self
                .stderr_lines
                .lock()
                .unwrap()
                .iter()
                .find(|line| matches(line))
            {
                return line.clone();
            }
            assert!(
                Instant::now() < deadline,
                "期待する stderr 行が出ない: {}",
                self.stderr_text()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// ここまでに溜まった stderr 行の複製。
    ///
    /// [`Self::wait_for_stderr_line`] は「出たか」しか見られないので、
    /// **行と行の順序**を判定したいときはこちらで丸ごと取ってから
    /// [`count_lines_between`] へ渡す。
    pub(super) fn stderr_snapshot(&self) -> Vec<String> {
        self.stderr_lines.lock().unwrap().clone()
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

/// サーバーのログ行から `名前=整数` を取り出す。
pub(super) fn number_field(line: &str, name: &str) -> u64 {
    let rest = line
        .split_once(&format!("{name}="))
        .unwrap_or_else(|| panic!("{name}= を含まない行: {line}"))
        .1;
    rest.split_whitespace()
        .next()
        .expect("値が空")
        .parse()
        .unwrap_or_else(|error| panic!("{name}= が整数でない ({error}): {line}"))
}

/// `start` に一致した行の**後**、`end` に一致した行の**前**にある `wanted` の数。
///
/// 「ロード中も受信が続いていた」は、行が出たかどうかでは判定できない。
/// ロード完了後にまとめて届いた場合も同じ行が出るからで、区別できるのは
/// **順序**だけ。時刻の sleep でログを推測せず、サーバー自身が出した
/// 開始行と終了行を目印にして、その間に挟まった行を数える。
pub(super) fn count_lines_between(
    lines: &[String],
    start: impl Fn(&str) -> bool,
    end: impl Fn(&str) -> bool,
    wanted: impl Fn(&str) -> bool,
) -> usize {
    let first = lines
        .iter()
        .position(|line| start(line))
        .unwrap_or_else(|| {
            panic!(
                "開始の目印が無い:
{}",
                lines.join(
                    "
"
                )
            )
        });
    let last = lines
        .iter()
        .skip(first + 1)
        .position(|line| end(line))
        .map(|offset| first + 1 + offset)
        .unwrap_or_else(|| {
            panic!(
                "終了の目印が無い:
{}",
                lines.join(
                    "
"
                )
            )
        });
    lines[first + 1..last]
        .iter()
        .filter(|line| wanted(line))
        .count()
}
