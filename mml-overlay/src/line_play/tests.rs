use super::*;

#[test]
fn a_chord_line_is_reported_as_a_chord() {
    let (status, events) = line_events("C");

    assert_eq!(
        status,
        LineStatus::Played {
            from_chord: true,
            note_count: 3,
        }
    );
    assert!(!events.is_empty());
}

#[test]
fn an_mml_line_is_reported_as_mml() {
    let (status, events) = line_events("cde");

    assert_eq!(
        status,
        LineStatus::Played {
            from_chord: false,
            note_count: 3,
        }
    );
    assert!(!events.is_empty());
}

/// 空行を通るたびにエラーが出ると、上下でフレーズを見て回るのが煩わしい。
/// 前の行を止めるためにイベントは空で返す。
#[test]
fn an_empty_line_is_silent_without_an_error() {
    assert_eq!(line_events("   "), (LineStatus::Idle, Vec::new()));
}

#[test]
fn a_broken_line_reports_the_error_and_stops_the_previous_line() {
    let (status, events) = line_events("r");

    assert!(matches!(status, LineStatus::Error(_)));
    assert!(events.is_empty());
}
