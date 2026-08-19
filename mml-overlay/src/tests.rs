use std::sync::Mutex;

/// 注入したログ sink へ実際に流れることの番人。
///
/// `log_line` は sink 未注入だとログを黙って捨てる。main.rs の
/// `cmrt_mml_overlay::set_log_sink` を消しても画面は動いてしまうので、ここで固定する。
#[test]
fn the_injected_log_sink_receives_the_line() {
    static RECEIVED: Mutex<Vec<String>> = Mutex::new(Vec::new());
    fn sink(line: &str) {
        RECEIVED.lock().unwrap().push(line.to_string());
    }
    super::set_log_sink(sink);

    super::log_line("sink test: hello".to_string());

    assert!(
        RECEIVED
            .lock()
            .unwrap()
            .iter()
            .any(|line| line == "mml-overlay: sink test: hello"),
        "注入した sink がログ行を受け取っていない"
    );
}
