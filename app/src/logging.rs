//! app 側のログ sink ポリシー。
//!
//! ログファイルへの書き込み / UI バッファ読み込み / native probe ロガー登録といった
//! 画面横断の純粋なファイル I/O は `cmrt-tui-core` の `logging` へ切り出した。
//! 従来の `crate::logging::{append_log_line, load_log_lines, install_native_probe_logger}`
//! パスは再エクスポートで維持する。
//!
//! ここに残すのは「app が各画面 crate（`cmrt-loop-browser` / `cmrt-realtime-play` 等）へ
//! 注入する sink」と、その sink が使う非同期ログ経路だけ。sink ポリシーは app が持つ。

use std::sync::mpsc;
#[cfg(not(test))]
use std::sync::OnceLock;

#[cfg(not(test))]
use cmrt_tui_core::logging::append_log_line_to_file;
pub(crate) use cmrt_tui_core::logging::install_native_probe_logger;

#[cfg(not(test))]
const ASYNC_LOG_CAPACITY: usize = 1_024;

#[cfg(not(test))]
pub(crate) fn append_global_log_line(line: impl AsRef<str>) {
    let _ = append_log_line_to_file(line.as_ref());
}

#[cfg(not(test))]
pub(crate) fn append_global_log_line_nonblocking(line: impl Into<String>) {
    static SENDER: OnceLock<Option<mpsc::SyncSender<String>>> = OnceLock::new();
    let sender = SENDER.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel::<String>(ASYNC_LOG_CAPACITY);
        std::thread::Builder::new()
            .name("cmrt-performance-log".to_string())
            .spawn(move || {
                while let Ok(line) = receiver.recv() {
                    let _ = append_log_line_to_file(&line);
                }
            })
            .ok()
            .map(|_| sender)
    });
    if let Some(sender) = sender {
        try_send_log_line(sender, line.into());
    }
}

#[cfg(test)]
pub(crate) fn append_global_log_line_nonblocking(line: impl Into<String>) {
    let _ = line.into();
}

/// 画面 crate（`cmrt-loop-browser` / `cmrt-realtime-play` 等）へ注入する同期ログ sink。
pub fn global_log_sink(line: &str) {
    #[cfg(not(test))]
    append_global_log_line(line);
    #[cfg(test)]
    let _ = line;
}

/// 画面 crate へ注入する非同期ログ sink（レンダースレッドを塞がない）。
pub fn nonblocking_log_sink(line: &str) {
    append_global_log_line_nonblocking(line.to_string());
}

/// 同一 process で使う play-server core の診断を、TUI-safe な非同期 sink へ向ける。
/// server binary はこれを呼ばず、既定の stderr を親 app に pipe させる。
pub fn install_embedded_core_log_sink() {
    cmrt_core::set_log_sink(nonblocking_log_sink);
}

fn try_send_log_line(sender: &mpsc::SyncSender<String>, line: String) {
    let _ = sender.try_send(line);
}

#[cfg(test)]
mod tests;
