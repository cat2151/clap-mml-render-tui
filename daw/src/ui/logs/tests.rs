use std::time::{Duration, UNIX_EPOCH};

use super::*;
use crate::logging::startup_log_line;

#[test]
fn startup_log_preserves_jst_text_and_highlights_only_time() {
    let message = startup_log_line(UNIX_EPOCH + Duration::from_secs(54_056));
    assert_eq!(message, "--- DAW 起動 1970-01-02 00:00:56 JST ---");

    let line = log_display_line(message.clone());

    assert_eq!(line.to_string(), message);
    assert_eq!(line.spans.len(), 3);
    assert_eq!(line.spans[0].style.fg, Some(MONOKAI_FG));
    assert_eq!(line.spans[1].content, "00:00:56");
    assert_eq!(line.spans[1].style.fg, Some(MONOKAI_PINK));
    assert_eq!(line.spans[2].style.fg, Some(MONOKAI_FG));
}

#[test]
fn shift_space_log_is_bold_pink() {
    let line = log_display_line(SHIFT_SPACE_LOG_LINE.to_owned());

    assert_eq!(line.to_string(), SHIFT_SPACE_LOG_LINE);
    assert_eq!(line.spans[0].style.fg, Some(MONOKAI_PINK));
    assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn incomplete_startup_logs_remain_readable() {
    for message in [
        "--- DAW 起動 ---",
        "--- DAW 起動 1970-01-02 JST ---",
        "--- DAW 起動 1970-01-02 00:00:56 JST",
        "--- DAW 起動 1970-01-02 00:00:56 UTC ---",
    ] {
        let line = log_display_line(message.to_owned());
        assert_eq!(line.to_string(), message);
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].style.fg, Some(MONOKAI_FG));
    }
}

#[test]
fn existing_log_colors_keep_error_priority() {
    for (message, expected_color) in [
        ("play: queue meas2", MONOKAI_PINK),
        ("play: start", MONOKAI_YELLOW),
        ("cache: rerender done track2", MONOKAI_GREEN),
        ("cache: start", MONOKAI_CYAN),
        ("play: queue failed", Color::Red),
        ("cache: rerender done error", Color::Red),
        ("✗ 演奏できません", Color::Red),
        ("通常のログ", MONOKAI_FG),
    ] {
        let line = log_display_line(message.to_owned());
        assert_eq!(line.to_string(), message);
        assert_eq!(line.spans[0].style.fg, Some(expected_color));
    }
}
