use std::sync::Mutex;

use super::*;

/// 注入された sink が受け取った行。sink は `fn` ポインタなのでキャプチャできず、static で受ける。
static CAPTURED: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn capture(line: &str) {
    CAPTURED.lock().unwrap().push(line.to_string());
}

/// sink 注入漏れは「ログが黙って消える」形で失敗するため、経路そのものを固定しておく。
#[test]
fn injected_sink_receives_prefixed_lines() {
    set_log_sink(capture);
    CAPTURED.lock().unwrap().clear();

    log_realtime_play_event("action=play retry=0");

    assert_eq!(
        CAPTURED.lock().unwrap().as_slice(),
        ["realtime-play: action=play retry=0".to_string()]
    );
}

#[test]
fn truncate_for_log_appends_ellipsis_beyond_the_limit() {
    assert_eq!(truncate_for_log("abcdef", 3), "abc...");
    assert_eq!(truncate_for_log("abc", 3), "abc");
}
