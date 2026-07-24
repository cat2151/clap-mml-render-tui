//! グローバルログへの書き込み sink（app からの注入で有効になる）。

use std::sync::OnceLock;

pub type LogSink = fn(&str);
static LOG_SINK: OnceLock<LogSink> = OnceLock::new();

/// app 起動時に、グローバルログ（`log/log.txt`）への書き込み関数を注入する。
/// 未注入の場合、この crate のログは黙って捨てられる。
pub fn set_log_sink(log: LogSink) {
    let _ = LOG_SINK.set(log);
}

/// sink 未注入時（テスト実行時を含む）は何も書かない。
fn log_line(message: &str) {
    if let Some(sink) = LOG_SINK.get() {
        sink(message);
    }
}

pub(crate) fn log_realtime_play_event(message: impl Into<String>) {
    log_line(&format!("realtime-play: {}", message.into()));
}

pub(crate) fn truncate_for_log(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index == max_chars {
            out.push_str("...");
            return out;
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests;
