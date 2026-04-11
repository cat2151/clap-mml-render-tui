use std::{
    collections::HashSet,
    sync::Arc,
    thread,
    time::{Duration, UNIX_EPOCH},
};

use super::{append_log_line_to_path, format_log_file_line_at};

fn split_log_file_line(line: &str) -> (&str, &str) {
    let (timestamp, message) = line.split_once("] ").expect("timestamp prefix");
    let timestamp = timestamp
        .strip_prefix('[')
        .expect("opening bracket in timestamp prefix");
    assert!(timestamp.ends_with(" UTC"));
    assert_eq!(timestamp.len(), 23);
    let bytes = timestamp.as_bytes();
    assert_eq!(bytes[4], b'-');
    assert_eq!(bytes[7], b'-');
    assert_eq!(bytes[10], b' ');
    assert_eq!(bytes[13], b':');
    assert_eq!(bytes[16], b':');
    assert_eq!(bytes[19], b' ');
    (timestamp, message)
}

#[test]
fn format_log_file_line_at_prefixes_human_readable_utc_timestamp() {
    assert_eq!(
        format_log_file_line_at("play: start", UNIX_EPOCH + Duration::from_secs(0)),
        "[1970-01-01 00:00:00 UTC] play: start"
    );
}

#[test]
fn append_log_line_to_path_keeps_concurrent_lines_intact() {
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_logging_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = tmp.join("log").join("log.txt");
    let thread_count = 8;
    let lines_per_thread = 16;
    let expected_lines: Arc<Vec<String>> = Arc::new(
        (0..thread_count)
            .flat_map(|thread_idx| {
                (0..lines_per_thread).map(move |line_idx| {
                    format!(
                        "thread-{thread_idx:02}-line-{line_idx:02}-{}",
                        "x".repeat(128)
                    )
                })
            })
            .collect(),
    );

    let mut handles = Vec::new();
    for thread_idx in 0..thread_count {
        let path = path.clone();
        let expected_lines = Arc::clone(&expected_lines);
        handles.push(thread::spawn(move || {
            for line_idx in 0..lines_per_thread {
                append_log_line_to_path(
                    &path,
                    &expected_lines[thread_idx * lines_per_thread + line_idx],
                )
                .unwrap();
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let actual_lines: Vec<String> = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(ToOwned::to_owned)
        .collect();
    let actual_messages: Vec<String> = actual_lines
        .iter()
        .map(|line| split_log_file_line(line).1.to_owned())
        .collect();
    let expected_set: HashSet<String> = expected_lines.iter().cloned().collect();
    let actual_set: HashSet<String> = actual_messages.iter().cloned().collect();

    assert_eq!(actual_lines.len(), thread_count * lines_per_thread);
    assert_eq!(actual_set, expected_set);

    std::fs::remove_dir_all(&tmp).ok();
}
