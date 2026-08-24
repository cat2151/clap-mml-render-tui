use std::time::Duration;

use super::{format_panic_report, panic_payload, try_send_log_line};

#[test]
fn full_async_log_queue_never_waits_for_capacity() {
    let (sender, _receiver) = std::sync::mpsc::sync_channel(1);
    sender.try_send("first".to_string()).unwrap();
    let started_at = std::time::Instant::now();

    try_send_log_line(&sender, "dropped".to_string());

    assert!(started_at.elapsed() < Duration::from_millis(50));
}

#[test]
fn panic_report_keeps_context_and_escapes_multiline_payloads() {
    let report = format_panic_report(
        "worker",
        "ThreadId(7)",
        "src/main.rs:12:3",
        "bad \"value\"\nnext",
        "frame one\nframe two",
    );

    assert!(report.contains("thread=\"worker\" thread_id=ThreadId(7)"));
    assert!(report.contains("location=\"src/main.rs:12:3\""));
    assert!(report.contains(r#"payload="bad \"value\"\nnext""#));
    assert!(report.contains("panic: backtrace frame one"));
    assert!(report.contains("panic: backtrace frame two"));
}

#[test]
fn panic_payload_accepts_string_and_str_payloads() {
    let borrowed: &(dyn std::any::Any + Send) = &"borrowed";
    let owned: &(dyn std::any::Any + Send) = &"owned".to_string();

    assert_eq!(panic_payload(borrowed), "borrowed");
    assert_eq!(panic_payload(owned), "owned");
}
