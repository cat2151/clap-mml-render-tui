//! アプリ全体の実メモリ使用量と OS の空き物理メモリ（画面横断で共有）。
//!
//! clap-mml-play-server の常駐プロセスは CLAP インスタンスを多数抱えるため、
//! 常時起動させたときにどこまでメモリを食うのかを help overlay で可視化する。
//! 計測はサーバへの問い合わせではなく OS のプロセス情報から行うので、
//! サーバ側の IPC プロトコルには一切触れない。
//!
//! help overlay 自体は待たせずに出したいので、[`request_refresh`] は計測を
//! バックグラウンドスレッドへ投げるだけで即座に返る。結果は共有状態に書かれ、
//! 次の描画フレームで [`overlay_lines`] が拾う（メインループは 50ms ごとに
//! 無条件で再描画するため、ポーリングは不要）。

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

/// 一度の計測で得られるメモリ情報。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemorySnapshot {
    /// cmrt 本体と clap-mml-play-server の常駐プロセスの Working Set の合計。
    pub total_working_set_bytes: u64,
    /// OS の空き物理メモリ。
    pub os_available_bytes: u64,
}

/// help overlay に出す値の状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// help overlay の先頭へ差し込む行（値 1 行 + 区切りの空行）を返す。
///
/// 副作用はないので、描画のたびに呼んでよい。
pub fn overlay_lines() -> Vec<Line<'static>> {
    format::overlay_lines(probe::reading())
}
