use std::{
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::Path,
    sync::{Arc, Mutex, OnceLock},
};

const MAX_LOG_LINES: usize = 256;

fn log_file_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn trim_log_lines(lines: &mut VecDeque<String>) {
    while lines.len() > MAX_LOG_LINES {
        lines.pop_front();
    }
}

fn append_log_line_to_path(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let _guard = log_file_lock()
        .lock()
        .expect("log file lock should not be poisoned");
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()
}

fn append_log_line_to_file(line: &str) -> std::io::Result<()> {
    let Some(path) = crate::config::log_file_path() else {
        return Ok(());
    };
    append_log_line_to_path(&path, line)
}

fn push_log_line(log_lines: &Arc<Mutex<VecDeque<String>>>, line: String) {
    let mut lines = log_lines.lock().unwrap();
    lines.push_back(line);
    trim_log_lines(&mut lines);
}

pub(crate) fn append_log_line(log_lines: &Arc<Mutex<VecDeque<String>>>, line: impl Into<String>) {
    let line = line.into();
    if let Err(err) = append_log_line_to_file(&line) {
        push_log_line(log_lines, format!("[log write error] {err}"));
    }
    push_log_line(log_lines, line);
}

pub(crate) fn load_log_lines() -> VecDeque<String> {
    let Some(path) = crate::config::log_file_path() else {
        return VecDeque::new();
    };
    let Ok(file) = File::open(path) else {
        return VecDeque::new();
    };

    let mut lines = VecDeque::new();
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if lines.len() == MAX_LOG_LINES {
            lines.pop_front();
        }
        lines.push_back(line);
    }
    lines
}

#[cfg(test)]
#[path = "tests/logging.rs"]
mod tests;
