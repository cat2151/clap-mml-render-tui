use super::*;
use std::time::Duration;

#[test]
fn reports_start_once_only_after_scheduled_time() {
    let logs = Arc::new(Mutex::new(VecDeque::new()));
    let progress = Progress::default();
    let now = Instant::now();
    assert!(!progress.poll(&logs, true, now));
    progress.schedule(now + Duration::from_secs(1));
    assert!(!progress.poll(&logs, true, now));
    assert!(logs.lock().unwrap().is_empty());
    assert!(progress.poll(&logs, true, now + Duration::from_secs(1)));
    assert!(progress.poll(&logs, true, now + Duration::from_secs(2)));
    let logs = logs.lock().unwrap();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("開始時刻に到達"));
}

#[test]
fn cancellation_suppresses_late_worker_completion_and_start() {
    let logs = Arc::new(Mutex::new(VecDeque::new()));
    let progress = Progress::default();
    progress.cancel(&logs, "停止・取消操作");
    progress.log(&logs, "演奏サーバーの起動が完了しました。");
    progress.schedule(Instant::now());
    assert!(progress.poll(&logs, true, Instant::now()));
    progress.cancel(&logs, "停止・取消操作");
    let logs = logs.lock().unwrap();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("取り消しました"));
}

#[test]
fn stopped_playback_does_not_report_start_even_after_deadline() {
    let logs = Arc::new(Mutex::new(VecDeque::new()));
    let progress = Progress::default();
    progress.schedule(Instant::now());
    assert!(progress.poll(&logs, false, Instant::now()));
    assert!(logs.lock().unwrap()[0].contains("開始前に停止"));
}
