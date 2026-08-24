//! デバッグログのファイル出力・UI バッファ読み込み・native probe ログ（notepad / DAW / keyboard 共通）。
//!
//! ここには「ログファイルへの書き込み / 読み込み」と「UI 用のインメモリバッファ操作」という、
//! 画面横断で共有する純粋なファイル I/O ユーティリティだけを置く。
//! `global_log_sink` / `nonblocking_log_sink` のような「app が各画面 crate へ注入する sink」は
//! app 側の `logging` に残す（sink ポリシーは app が持つ）。

use std::{
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, TryLockError},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_LOG_LINES: usize = 256;
const JST_OFFSET_SECONDS: i64 = 9 * 60 * 60;

fn log_file_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn trim_log_lines(lines: &mut VecDeque<String>) {
    while lines.len() > MAX_LOG_LINES {
        lines.pop_front();
    }
}

fn unix_seconds_floor(now: SystemTime) -> i64 {
    match now.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as i64,
        Err(err) => {
            let duration = err.duration();
            let seconds = duration.as_secs() as i64;
            if duration.subsec_nanos() == 0 {
                -seconds
            } else {
                -seconds - 1
            }
        }
    }
}

fn format_jst_timestamp(now: SystemTime) -> String {
    let unix_seconds = unix_seconds_floor(now).saturating_add(JST_OFFSET_SECONDS);
    let days = unix_seconds.div_euclid(86_400);
    let seconds_of_day = unix_seconds.rem_euclid(86_400);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} JST")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

fn format_log_file_line_at(line: &str, now: SystemTime) -> String {
    format!("[{}] {line}", format_jst_timestamp(now))
}

fn strip_log_file_timestamp_prefix(line: &str) -> &str {
    let bytes = line.as_bytes();
    if bytes.len() < 26
        || bytes[0] != b'['
        || bytes[5] != b'-'
        || bytes[8] != b'-'
        || bytes[11] != b' '
        || bytes[14] != b':'
        || bytes[17] != b':'
        || bytes[20] != b' '
        || bytes[24] != b']'
        || bytes[25] != b' '
    {
        return line;
    }

    let timezone = &bytes[21..24];
    if timezone != b"UTC" && timezone != b"JST" {
        return line;
    }

    for idx in [1usize, 2, 3, 4, 6, 7, 9, 10, 12, 13, 15, 16, 18, 19] {
        if !bytes[idx].is_ascii_digit() {
            return line;
        }
    }

    &line[26..]
}

fn append_log_line_to_path(path: &Path, line: &str) -> std::io::Result<()> {
    let _guard = log_file_lock()
        .lock()
        .expect("log file lock should not be poisoned");
    append_log_lines_without_lock(path, line)
}

/// panic hookは通常ロガー自身のpanicからも呼ばれうる。通常mutexを待つと同じthreadで
/// deadlockするため、取得できないときだけ同じdirectoryの`panic.log`へ退避する。
fn append_panic_report_to_path(path: &Path, report: &str) -> std::io::Result<()> {
    let _guard = match log_file_lock().try_lock() {
        Ok(guard) => guard,
        Err(TryLockError::Poisoned(error)) => error.into_inner(),
        Err(TryLockError::WouldBlock) => {
            let fallback = path.with_file_name("panic.log");
            return append_log_lines_without_lock(&fallback, report);
        }
    };
    append_log_lines_without_lock(path, report)
}

fn append_log_lines_without_lock(path: &Path, lines: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let now = SystemTime::now();
    let mut formatted = String::new();
    let mut source_lines = lines.lines().peekable();
    if source_lines.peek().is_none() {
        formatted.push_str(&format_log_file_line_at("", now));
        formatted.push('\n');
    } else {
        for line in source_lines {
            formatted.push_str(&format_log_file_line_at(line, now));
            formatted.push('\n');
        }
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(formatted.as_bytes())?;
    file.flush()
}

fn append_log_line_to_optional_path(path: Option<PathBuf>, line: &str) -> std::io::Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    append_log_line_to_path(&path, line)
}

/// ログファイル（`log/log.txt`）へ 1 行追記する。app 側の sink（`global_log_sink` 等）からも使う。
pub fn append_log_line_to_file(line: &str) -> std::io::Result<()> {
    append_log_line_to_optional_path(cmrt_runtime::log_file_path(), line)
}

/// panic payloadとbacktraceを終了前に同期書き込みする。
///
/// 通常ログのmutexを待てない場合は`log/panic.log`へフォールバックする。
pub fn append_panic_report_to_file(report: &str) -> std::io::Result<()> {
    let Some(path) = cmrt_runtime::log_file_path() else {
        return Ok(());
    };
    append_panic_report_to_path(&path, report)
}

fn append_native_probe_log_line_to_file(line: &str) -> std::io::Result<()> {
    append_log_line_to_optional_path(cmrt_runtime::native_probe_log_file_path(), line)
}

/// UI 表示用のインメモリバッファへだけ 1 行 push する。
///
/// ログファイルの書き先は実ユーザーの `log/log.txt` 固定なので、テストからは
/// こちらだけを使う（[`append_log_line`] はファイルにも書く）。
pub fn append_log_line_in_memory(
    log_lines: &Arc<Mutex<VecDeque<String>>>,
    line: impl Into<String>,
) {
    let mut lines = log_lines.lock().unwrap();
    lines.push_back(line.into());
    trim_log_lines(&mut lines);
}

/// ログファイルへ追記しつつ、UI 表示用のインメモリバッファへも 1 行 push する。
pub fn append_log_line(log_lines: &Arc<Mutex<VecDeque<String>>>, line: impl Into<String>) {
    let line = line.into();
    if let Err(err) = append_log_line_to_file(&line) {
        append_log_line_in_memory(log_lines, format!("[log write error] {err}"));
    }
    append_log_line_in_memory(log_lines, line);
}

fn append_native_probe_log_line(line: impl AsRef<str>) {
    let _ = append_native_probe_log_line_to_file(line.as_ref());
}

/// cmrt-core の native render probe ログを、専用ログファイルへ流すロガーを登録する。
pub fn install_native_probe_logger() {
    let logger: cmrt_core::NativeProbeLogger =
        Arc::new(|line: &str| append_native_probe_log_line(line));
    cmrt_core::set_native_probe_logger(Some(logger));
}

fn load_log_lines_from_path(path: &Path) -> VecDeque<String> {
    let Ok(file) = File::open(path) else {
        return VecDeque::new();
    };

    let mut lines = VecDeque::new();
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if lines.len() == MAX_LOG_LINES {
            lines.pop_front();
        }
        lines.push_back(strip_log_file_timestamp_prefix(&line).to_string());
    }
    lines
}

/// 起動時、ログファイルの末尾から UI 表示用の初期バッファを構築する。
pub fn load_log_lines() -> VecDeque<String> {
    let Some(path) = cmrt_runtime::log_file_path() else {
        return VecDeque::new();
    };
    load_log_lines_from_path(&path)
}

#[cfg(test)]
mod tests;
