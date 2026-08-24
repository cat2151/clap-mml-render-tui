//! app 側のログ sink ポリシー。
//!
//! ログファイルへの書き込み / UI バッファ読み込み / native probe ロガー登録といった
//! 画面横断の純粋なファイル I/O は `cmrt-tui-core` の `logging` へ切り出した。
//! 従来の `crate::logging::{append_log_line, load_log_lines, install_native_probe_logger}`
//! パスは再エクスポートで維持する。
//!
//! ここに残すのは「app が各画面 crate（`cmrt-loop-browser` / `cmrt-realtime-play` 等）へ
//! 注入する sink」と、その sink が使う非同期ログ経路だけ。sink ポリシーは app が持つ。

#[cfg(not(test))]
use std::sync::OnceLock;
use std::{
    any::Any,
    backtrace::Backtrace,
    panic::{self, AssertUnwindSafe, PanicHookInfo},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Once,
    },
};

#[cfg(not(test))]
use cmrt_tui_core::logging::append_log_line_to_file;
use cmrt_tui_core::logging::append_panic_report_to_file;
pub(crate) use cmrt_tui_core::logging::install_native_probe_logger;

#[cfg(not(test))]
const ASYNC_LOG_CAPACITY: usize = 1_024;

static PANIC_HOOK_ACTIVE: AtomicBool = AtomicBool::new(false);

/// 未処理panicを同期ログへ残してから、標準hookのstderr出力も維持する。
pub fn install_panic_log_hook() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if !PANIC_HOOK_ACTIVE.swap(true, Ordering::SeqCst) {
                let result = panic::catch_unwind(AssertUnwindSafe(|| {
                    let report = panic_report(info);
                    if let Err(error) = append_panic_report_to_file(&report) {
                        eprintln!("panic log write failed: {error}");
                    }
                }));
                PANIC_HOOK_ACTIVE.store(false, Ordering::SeqCst);
                if result.is_err() {
                    eprintln!("panic log formatting failed");
                }
            }
            previous(info);
        }));
    });
}

fn panic_report(info: &PanicHookInfo<'_>) -> String {
    let thread = std::thread::current();
    let thread_name = thread.name().unwrap_or("<unnamed>");
    let thread_id = format!("{:?}", thread.id());
    let location = info.location().map_or_else(
        || "<unknown>".to_string(),
        |location| {
            format!(
                "{}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            )
        },
    );
    let payload = panic_payload(info.payload());
    let backtrace = Backtrace::force_capture().to_string();
    format_panic_report(thread_name, &thread_id, &location, &payload, &backtrace)
}

fn panic_payload(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "<non-string panic payload>".to_string())
}

fn format_panic_report(
    thread_name: &str,
    thread_id: &str,
    location: &str,
    payload: &str,
    backtrace: &str,
) -> String {
    let mut lines = vec![format!(
        "panic: event=unhandled thread=\"{}\" thread_id={} location=\"{}\" payload=\"{}\"",
        escape_log_value(thread_name),
        thread_id,
        escape_log_value(location),
        escape_log_value(payload),
    )];
    lines.extend(
        backtrace
            .lines()
            .map(|line| format!("panic: backtrace {line}")),
    );
    lines.join("\n")
}

fn escape_log_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('"', "\\\"")
}

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
