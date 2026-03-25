use std::{collections::HashSet, sync::Arc, thread};

use super::append_log_line_to_path;

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
    let expected_set: HashSet<String> = expected_lines.iter().cloned().collect();
    let actual_set: HashSet<String> = actual_lines.iter().cloned().collect();

    assert_eq!(actual_lines.len(), thread_count * lines_per_thread);
    assert_eq!(actual_set, expected_set);

    std::fs::remove_dir_all(&tmp).ok();
}
