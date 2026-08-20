//! server プロセスが落ちた理由の保持と、同じ理由で spawn し続けないための打ち切り。
//!
//! 背景: install 済みの古い server exe を掴むと、いまの config を拒否して即死する。
//! それでも `wait_for_port_locked` はポーリングのたびに spawn し直していたため、
//! 1 セッションで数百プロセスを作り、ユーザーから見える情報は「無音」だけだった
//! （落ちた理由は子の stderr にあり、log.txt を読まないと分からなかった）。
//!
//! ここが持つのは 3 つ。
//! - 子の stderr の末尾（= 落ちた理由）を、child を捨てる前に拾うための入れ物
//! - その理由を、エラー文と UI 表示の両方へ流し込める形にしたもの
//! - 同じ理由で spawn し続けないための門番（[`ExitLatch`]）

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

/// 保持する stderr の行数。config エラーは 1 行だが、panic なら先頭数行が要る。
const STDERR_TAIL_LINES: usize = 8;

/// 子が終了したあと、reader スレッドの読み切りを待つ上限。
///
/// `try_wait` が終了を返した時点では、最後の行がまだパイプに残っていることがある。
/// その 1 行が「なぜ落ちたか」そのものなので少しだけ待つ。孫プロセスが stderr を
/// 握ったまま残っても止まらないよう、上限を置く。
const STDERR_DRAIN_WAIT: Duration = Duration::from_millis(300);

/// 読み切り待ちのポーリング間隔。
const STDERR_DRAIN_POLL: Duration = Duration::from_millis(10);

/// ポートが開かないまま連続で終了したとき、これ以上 spawn しない回数。
pub(crate) const MAX_CONSECUTIVE_EXITS: usize = 3;

/// 打ち切りを解除するまでの間隔。これだけ間が空いたら、ユーザーが config を
/// 直したかもしれないので、もう一度だけ試す（直っていなければまたすぐ打ち切る）。
pub(crate) const EXIT_LATCH_RESET: Duration = Duration::from_secs(30);

const NO_STDERR: &str = "(stderr に出力なし)";

/// 直近に server が落ちた理由。エラー文と UI 表示の両方がこれを文字列化する。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerStartupFailure {
    /// 起動しようとした実体。config 指定ならそのコマンド文字列。
    /// **どの exe を掴んだか**が、この種の事故で最初に要る情報。
    pub exe: String,
    /// 終了コード。取れないときは `None`。
    pub exit_code: Option<i32>,
    /// 子の stderr の末尾。ここに落ちた理由が書いてある。
    pub stderr_tail: Vec<String>,
}

impl ServerStartupFailure {
    /// エラー文 1 行。`anyhow` に載せてログにも UI にも流れる。
    pub fn message(&self) -> String {
        format!(
            "realtime play server が起動できません (exe=\"{}\", exit={}): {}",
            self.exe,
            self.exit_code_text(),
            self.detail_text()
        )
    }

    /// UI 用の複数行。1 行目に「何が起きたか」、2 行目に掴んだ実体を置く。
    /// 狭い端末で下が切れても、この 2 行が残れば切り分けられる。
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            "play server が起動できません".to_owned(),
            format!("exe=\"{}\"", self.exe),
            format!("exit={}", self.exit_code_text()),
        ];
        if self.stderr_tail.is_empty() {
            lines.push(NO_STDERR.to_owned());
        } else {
            lines.extend(self.stderr_tail.iter().cloned());
        }
        lines
    }

    fn exit_code_text(&self) -> String {
        match self.exit_code {
            Some(code) => code.to_string(),
            None => "不明".to_owned(),
        }
    }

    fn detail_text(&self) -> String {
        if self.stderr_tail.is_empty() {
            NO_STDERR.to_owned()
        } else {
            self.stderr_tail.join(" / ")
        }
    }
}

/// 子の stderr の末尾を持ち回るための共有ハンドル。
///
/// 書くのは reader スレッド、読むのは「子が落ちたと気づいた側」なので共有する。
#[derive(Clone, Debug, Default)]
pub(crate) struct StderrCapture {
    tail: Arc<Mutex<StderrTail>>,
    finished: Arc<AtomicBool>,
}

impl StderrCapture {
    pub(crate) fn push(&self, line: String) {
        self.tail.lock().unwrap().push(line);
    }

    /// reader スレッドが stderr を読み切ったことを知らせる。
    pub(crate) fn mark_finished(&self) {
        self.finished.store(true, Ordering::Release);
    }

    /// 読み切りを待ってから末尾を返す。
    pub(crate) fn drain_snapshot(&self) -> Vec<String> {
        self.drain_snapshot_within(STDERR_DRAIN_WAIT)
    }

    fn drain_snapshot_within(&self, wait: Duration) -> Vec<String> {
        let deadline = Instant::now() + wait;
        while !self.finished.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(STDERR_DRAIN_POLL);
        }
        self.tail.lock().unwrap().snapshot()
    }
}

/// stderr の末尾だけを残すリングバッファ。
#[derive(Debug, Default)]
struct StderrTail {
    lines: VecDeque<String>,
}

impl StderrTail {
    fn push(&mut self, line: String) {
        if self.lines.len() == STDERR_TAIL_LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    fn snapshot(&self) -> Vec<String> {
        self.lines.iter().cloned().collect()
    }
}

/// 「同じ理由で落ち続けるサーバーを spawn し直さない」門番。
///
/// 落ちるたびに数え、ポートが開いたら 0 に戻す。[`EXIT_LATCH_RESET`] だけ間が空いたときも
/// 0 に戻す（ユーザーが config を直したかもしれないので、永久に諦めたままにはしない）。
#[derive(Debug, Default)]
pub(crate) struct ExitLatch {
    consecutive_exits: usize,
    last_exit_at: Option<Instant>,
}

impl ExitLatch {
    pub(crate) fn record_exit(&mut self, now: Instant) {
        self.consecutive_exits += 1;
        self.last_exit_at = Some(now);
    }

    /// サーバーが起動できたときに呼ぶ。
    pub(crate) fn reset(&mut self) {
        self.consecutive_exits = 0;
        self.last_exit_at = None;
    }

    /// いま spawn を止めるべきか。止めないと判断したときは数え直しから始める。
    pub(crate) fn engaged(&mut self, now: Instant) -> bool {
        let Some(last_exit_at) = self.last_exit_at else {
            return false;
        };
        if now.duration_since(last_exit_at) >= EXIT_LATCH_RESET {
            self.reset();
            return false;
        }
        self.consecutive_exits >= MAX_CONSECUTIVE_EXITS
    }
}

#[cfg(test)]
mod tests;
