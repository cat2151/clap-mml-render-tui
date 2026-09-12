//! アプリ全体の実メモリ使用量と OS の空き物理メモリ（help overlay に出す。画面横断で共有）。
//!
//! 計測は OS のプロセス情報から行い、play server の IPC プロトコルには触れない。
//! 合計だけでは「TUI 本体とサーバのどちらが食っているか」が分からないので、プロセスごとの
//! Working Set も並べる（CLAP インスタンスはサーバの 1 プロセスに同居するので、
//! 1 インスタンスあたりの値までは出ない）。
//!
//! [`request_refresh`] は計測をバックグラウンドへ投げて即座に返り、結果は次の描画フレームで
//! [`overlay_lines`] が拾う（メインループは 50ms ごとに無条件で再描画するので、ポーリング不要）。
//! 瞬間の Working Set はハズレ値が出るので、[`overlay_lines`] が 1 秒間隔で再計測を投げ直す。

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

/// バックグラウンド計測を開始する。すでに計測中なら何もしない（連打しても計測スレッドは増えない）。
pub fn request_refresh() {
    probe::request_refresh(platform::measure);
}

/// help overlay の先頭へ差し込む行（合計 1 行 + 内訳 + 区切りの空行）を返す。
///
/// 描画のたびに呼んでよい。ブロックはせず、再計測をバックグラウンドへ投げるだけ。
/// help を出している間だけ呼ばれるので、「help 表示中は更新、閉じている間は計測しない」になる。
pub fn overlay_lines() -> Vec<Line<'static>> {
    request_refresh();
    format::overlay_lines(probe::reading())
}
