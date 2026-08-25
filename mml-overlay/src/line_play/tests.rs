use super::*;

#[test]
fn a_chord_line_is_reported_as_a_chord() {
    let (status, performance) = line_events("C");

    assert_eq!(
        status,
        LineStatus::Played {
            from_chord: true,
            note_count: 3,
        }
    );
    assert!(!performance.is_silent());
}

#[test]
fn an_mml_line_is_reported_as_mml() {
    let (status, performance) = line_events("cde");

    assert_eq!(
        status,
        LineStatus::Played {
            from_chord: false,
            note_count: 3,
        }
    );
    assert!(!performance.is_silent());
}

/// 空行を通るたびにエラーが出ると、上下でフレーズを見て回るのが煩わしい。
/// 前の行を止めるためにイベントは空で返す。
#[test]
fn an_empty_line_is_silent_without_an_error() {
    assert_eq!(
        line_events("   "),
        (LineStatus::Idle, LinePerformance::silent())
    );
}

#[test]
fn a_broken_line_reports_the_error_and_stops_the_previous_line() {
    let (status, performance) = line_events("r");

    assert!(matches!(status, LineStatus::Error(_)));
    assert!(performance.is_silent());
}

/// 繰り返すには 1 周の長さが要る。イベント列だけでは「いつ次の周を積むか」が決まらない。
#[test]
fn a_line_reports_how_long_one_cycle_is() {
    let (_, performance) = line_events("cde");

    assert!(
        performance.loop_seconds > 0.0,
        "loop_seconds={}",
        performance.loop_seconds
    );
    // 最後のイベント（3 音目の note off）までが 1 周。
    let last_event_seconds = performance
        .events
        .last()
        .expect("a played line has events")
        .seconds;
    assert!(
        (performance.loop_seconds - last_event_seconds).abs() < 1e-9,
        "loop_seconds={} last={last_event_seconds}",
        performance.loop_seconds
    );
}

/// **罠**: `duration_seconds` は「最後のイベントまで」なので行末の休符は落ちる。
/// 直すなら chord 側なので、ここでは現状を固定して次の人が気づけるようにしておく。
#[test]
fn a_trailing_rest_does_not_lengthen_the_cycle() {
    let (_, without_rest) = line_events("cde");
    let (_, with_rest) = line_events("cder");

    assert_eq!(with_rest.loop_seconds, without_rest.loop_seconds);
}

/// `Ctrl+Space` は端末によって 2 通りの綴りで届く。どちらも同じ意味。
///
/// 判定を overlay 本体と音色選択で二重に書くと、片方だけ直す事故が起きる。
/// ここが唯一の定義であることを固定する。
#[test]
fn ctrl_space_arrives_with_two_spellings() {
    assert!(is_replay_key(KeyEvent::new(
        KeyCode::Char(' '),
        KeyModifiers::CONTROL
    )));
    assert!(is_replay_key(KeyEvent::new(
        KeyCode::Char('\0'),
        KeyModifiers::CONTROL
    )));
}

/// Ctrl の無い空白はただの打鍵。行を鳴らし直すキーではない。
#[test]
fn a_plain_space_is_not_a_replay_key() {
    assert!(!is_replay_key(KeyEvent::new(
        KeyCode::Char(' '),
        KeyModifiers::NONE
    )));
}
