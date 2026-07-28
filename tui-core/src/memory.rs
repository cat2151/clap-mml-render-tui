//! アプリ全体の実メモリ使用量と OS の空き物理メモリ（画面横断で共有）。
//!
//! clap-mml-play-server の常駐プロセスは CLAP インスタンスを多数抱えるため、
//! 常時起動させたときにどこまでメモリを食うのかを help overlay で可視化する。
//! 計測はサーバへの問い合わせではなく OS のプロセス情報から行うので、
//! サーバ側の IPC プロトコルには一切触れない。
//!
//! 合計だけでは「TUI 本体とサーバのどちらが食っているのか」が分からないため、
//! プロセスごとの Working Set も内訳として並べる。なお 16 CLAP インスタンスは
//! realtime play server の 1 プロセス内に同居しているので、この内訳から
//! 「1 インスタンスあたり何 MB か」までは分からない。
//!
//! help overlay 自体は待たせずに出したいので、[`request_refresh`] は計測を
//! バックグラウンドスレッドへ投げるだけで即座に返る。結果は共有状態に書かれ、
//! 次の描画フレームで [`overlay_lines`] が拾う（メインループは 50ms ごとに
//! 無条件で再描画するため、ポーリングは不要）。
//!
//! 値は瞬間の Working Set なので一時的にハズレ値が出る。表示したまま固まると
//! ユーザーが誤読するため、[`overlay_lines`] が描画のついでに 1 秒間隔で
//! 再計測を投げ直し、help を開いている間ずっと値が更新され続けるようにする。

use ratatui::text::Line;

mod format;
mod probe;

#[cfg(not(windows))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
use unsupported as platform;
#[cfg(windows)]
use windows as platform;

#[cfg(test)]
mod tests;

/// 計測対象プロセス 1 つぶん。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessMemory {
    /// プロセス一覧から取った exe 名（例: `cmrt.exe`）。
    pub name: String,
    pub pid: u32,
    pub working_set_bytes: u64,
}

/// 一度の計測で得られるメモリ情報。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySnapshot {
    /// 内訳。自プロセスが先頭で、以降は常駐サーバ。
    pub processes: Vec<ProcessMemory>,
    /// [`Self::processes`] の Working Set 合計。
    pub total_working_set_bytes: u64,
    /// OS の空き物理メモリ。
    pub os_available_bytes: u64,
}

/// help overlay に出す値の状態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MemoryReading {
    /// バックグラウンド計測がまだ完了していない。
    Measuring,
    Ready(MemorySnapshot),
    /// 計測に失敗した（非対応プラットフォームを含む）。
    Unavailable,
}

/// バックグラウンド計測を開始する。help overlay を開いた瞬間に呼ぶ。
///
/// すでに計測中なら何もしないので、連打しても計測スレッドは増えない。
pub fn request_refresh() {
    probe::request_refresh(platform::measure);
}

/// help overlay の先頭へ差し込む行（合計 1 行 + 内訳 + 区切りの空行）を返す。
///
/// 内訳が出るのは計測が済んだあとだけで、計測中・取得不可のときは
/// 合計行と空行の 2 行のまま。
///
/// 描画のたびに呼んでよい。ブロックはせず、前回計測から 1 秒以上経っていれば
/// 次フレーム以降に反映される再計測をバックグラウンドへ投げるだけ。
pub fn overlay_lines() -> Vec<Line<'static>> {
    // help を出している間だけこの関数が呼ばれるので、ここで再計測を投げると
    // 「help 表示中は 1 秒ごとに更新、閉じている間は計測しない」になる。
    request_refresh();
    format::overlay_lines(probe::reading())
}
