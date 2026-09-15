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

#[test]
fn a_chord_line_uses_the_supplied_key_and_directive() {
    let (status, performance) = chord_line_events("II", "key:G", "close", "");
    let pitches = performance
        .events
        .iter()
        .filter(|event| event.message[0] == NOTE_ON)
        .map(|event| event.message[1])
        .collect::<Vec<_>>();

    assert_eq!(
        status,
        LineStatus::Played {
            from_chord: true,
            note_count: 3,
        }
    );
    assert_eq!(pitches, vec![69, 73, 76]);
}

#[test]
fn a_chord_chart_line_uses_the_supplied_key() {
    let (_, in_c) = chord_chart_line_events("II", Some("Key=C"));
    let (status, in_g) = chord_chart_line_events("II", Some("Key=G"));
    let pitches = |performance: &LinePerformance| {
        performance
            .events
            .iter()
            .filter(|event| event.message[0] == NOTE_ON)
            .map(|event| event.message[1])
            .collect::<Vec<_>>()
    };

    assert_eq!(pitches(&in_c), vec![62, 66, 69]);
    assert_eq!(pitches(&in_g), vec![69, 73, 76]);
    assert_eq!(
        status,
        LineStatus::Played {
            from_chord: true,
            note_count: 3,
        }
    );
}

#[test]
fn a_chord_chart_progression_is_not_wrapped_like_a_daw_cell() {
    let (_, progression) = chord_chart_line_events("I V", Some("Key=C"));
    let (_, cell) = chord_line_events("I V", "Key=C", "", "");

    assert!(
        progression.loop_seconds > cell.loop_seconds,
        "progression={} cell={}",
        progression.loop_seconds,
        cell.loop_seconds
    );
}

#[test]
fn a_broken_chord_chart_line_never_falls_back_to_mml() {
    let (status, performance) = chord_chart_line_events("cde", Some("Key=G"));

    assert!(matches!(status, LineStatus::Error(error) if error.contains("コード変換に失敗")));
    assert!(performance.is_silent());
}

#[test]
fn auto_voiced_chord_chart_line_uses_exact_voicings_without_the_bass() {
    let parsed = cmrt_chord::parse_chord_progression("Key:C I-IV-V-I").unwrap();
    let voicings = cmrt_chord::auto_voice(parsed.chords(), None);

    let (status, performance) =
        auto_voiced_chord_chart_line_events("I-IV-V-I", Some("Key=C"), &voicings, None);

    assert_eq!(
        status,
        LineStatus::Played {
            from_chord: true,
            note_count: 12,
        }
    );
    let pitches = performance
        .events
        .iter()
        .filter(|event| event.message[0] == NOTE_ON)
        .map(|event| event.message[1])
        .collect::<Vec<_>>();
    assert_eq!(
        pitches,
        voicings
            .iter()
            .flat_map(|voicing| voicing.notes.iter().copied())
            .collect::<Vec<_>>()
    );
    assert_eq!(pitches.len(), 12, "bass は chord patch へ混ぜない");
}

#[test]
fn one_chord_preview_keeps_the_voicing_from_its_progression_position() {
    let parsed = cmrt_chord::parse_chord_progression("Key:C I-IV-V-I").unwrap();
    let voicings = cmrt_chord::auto_voice(parsed.chords(), None);

    let (status, performance) =
        auto_voiced_chord_chart_line_events("I-IV-V-I", Some("Key=C"), &voicings, Some(2));

    assert_eq!(
        status,
        LineStatus::Played {
            from_chord: true,
            note_count: voicings[2].notes.len(),
        }
    );
    let pitches = performance
        .events
        .iter()
        .filter(|event| event.message[0] == NOTE_ON)
        .map(|event| event.message[1])
        .collect::<Vec<_>>();
    assert_eq!(pitches, voicings[2].notes);
    assert_eq!(performance.loop_seconds, 2.0);
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
