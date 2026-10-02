type LogSink = fn(&str);
static LOG_SINK: std::sync::OnceLock<LogSink> = std::sync::OnceLock::new();

/// app 起動時に、グローバルログ（`log/log.txt`）への書き込み関数を注入する。
/// 未注入の場合（テスト実行時を含む）、この crate のログは黙って捨てられる。
pub fn set_log_sink(log: LogSink) {
    let _ = LOG_SINK.set(log);
}

/// 1 行 1 事象。`keyboard: ` を前に付けて書く。
pub(crate) fn log_line(message: &str) {
    if let Some(sink) = LOG_SINK.get() {
        sink(&format!("keyboard: {message}"));
    }
}
