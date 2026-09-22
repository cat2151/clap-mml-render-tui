use std::time::Duration;

use super::slow_operation_log_line;

#[test]
fn logs_only_when_elapsed_time_is_over_one_hundred_milliseconds() {
    assert!(slow_operation_log_line("draw", Duration::from_millis(100), None).is_none());

    let line = slow_operation_log_line("draw", Duration::from_micros(100_001), None).unwrap();

    assert!(line.contains("stage=draw"), "line={line}");
    assert!(line.contains("elapsed_ms=100.001"), "line={line}");
}

#[test]
fn includes_escaped_context_for_key_and_mode_identification() {
    let line = slow_operation_log_line(
        "input-to-draw",
        Duration::from_millis(250),
        Some("mode=EffectChainAdd key=Char('j')\nnext"),
    )
    .unwrap();

    assert!(
        line.contains(r#"context="mode=EffectChainAdd key=Char('j')\nnext""#),
        "line={line}"
    );
}
