use std::{
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    sync::{Arc, Mutex},
};

const MAX_LOG_LINES: usize = 256;

fn trim_log_lines(lines: &mut VecDeque<String>) {
    while lines.len() > MAX_LOG_LINES {
        lines.pop_front();
    }
}

fn append_log_line_to_file(line: &str) -> std::io::Result<()> {
    let Some(path) = crate::config::log_file_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    file.flush()
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
