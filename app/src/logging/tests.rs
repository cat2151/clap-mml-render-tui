use std::time::Duration;

use super::try_send_log_line;

#[test]
fn full_async_log_queue_never_waits_for_capacity() {
    let (sender, _receiver) = std::sync::mpsc::sync_channel(1);
    sender.try_send("first".to_string()).unwrap();
    let started_at = std::time::Instant::now();

    try_send_log_line(&sender, "dropped".to_string());

    assert!(started_at.elapsed() < Duration::from_millis(50));
}
