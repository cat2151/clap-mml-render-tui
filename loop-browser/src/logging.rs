use std::sync::OnceLock;

type LogSink = fn(&str);
static LOG_SINK: OnceLock<LogSink> = OnceLock::new();
static PERF_LOG_SINK: OnceLock<LogSink> = OnceLock::new();

/// app 起動時に、グローバルログ（`log/log.txt`）への書き込み関数を注入する。
/// `log` は同期書き込み、`perf_log` はレンダースレッドを塞がない非同期書き込みを想定する。
pub fn set_log_sinks(log: LogSink, perf_log: LogSink) {
    let _ = LOG_SINK.set(log);
    let _ = PERF_LOG_SINK.set(perf_log);
}

pub(crate) fn log_line(message: &str) {
    if let Some(sink) = LOG_SINK.get() {
        sink(message);
    }
}

pub(crate) fn perf_log_line(message: &str) {
    if let Some(sink) = PERF_LOG_SINK.get() {
        sink(message);
    }
}
