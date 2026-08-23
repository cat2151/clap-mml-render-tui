//! notepad crate から app のグローバルログへ接続する sink。

type LogSink = fn(&str);
static LOG_SINK: std::sync::OnceLock<LogSink> = std::sync::OnceLock::new();

/// app 起動時に、グローバルログ（`log/log.txt`）への書き込み関数を注入する。
/// 未注入の場合、この crate のログは黙って捨てられる。
pub fn set_log_sink(log: LogSink) {
    let _ = LOG_SINK.set(log);
}

pub(crate) fn log_notepad_event(message: impl Into<String>) {
    if let Some(sink) = LOG_SINK.get() {
        sink(&format!("notepad: {}", message.into()));
    }
}
