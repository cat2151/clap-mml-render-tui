//! 複数行の入力と、行ごとの演奏。

use super::*;

fn played_pitches(action: &MmlOverlayAction) -> Vec<u8> {
    let MmlOverlayAction::PlayLine { events, .. } = action else {
        panic!("expected a line playback, got {action:?}");
    };
    events
        .iter()
        .filter(|event| event.message[0] == NOTE_ON)
        .map(|event| event.message[1])
        .collect()
}

#[test]
fn enter_starts_a_new_line() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(press(KeyCode::Enter), now);
    type_chars(&mut overlay, "efg", now);

    assert_eq!(overlay.value(), "cde\nefg");
}

/// 上下キーは「その行のすべて」を鳴らす。
#[test]
fn moving_to_another_line_plays_the_whole_line() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(press(KeyCode::Enter), now);
    type_chars(&mut overlay, "gab", now);

    let action = overlay.handle_key(press(KeyCode::Up), now);

    assert_eq!(played_pitches(&action), vec![60, 62, 64]);
    assert_eq!(
        overlay.line_status(),
        &LineStatus::Played {
            from_chord: false,
            note_count: 3,
        }
    );
}

/// 行は独立して解釈する。上の行のオクターブ指定は下の行へ持ち越さない。
#[test]
fn a_line_does_not_inherit_the_octave_of_the_line_above() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, ">>c", now);
    overlay.handle_key(press(KeyCode::Enter), now);
    type_chars(&mut overlay, "c", now);

    let action = overlay.handle_key(press(KeyCode::Up), now);
    let shifted = played_pitches(&action);
    let action = overlay.handle_key(press(KeyCode::Down), now);

    assert_eq!(played_pitches(&action), vec![60]);
    assert_ne!(shifted, vec![60]);
}

/// 打鍵の 1 音も、その行だけを見て決める。
#[test]
fn typing_on_a_line_ignores_the_line_above() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, ">>", now);
    overlay.handle_key(press(KeyCode::Enter), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('c')), now),
        MmlOverlayAction::Send(vec![[0x90, 60, 127]])
    );
}

/// コード表記は打った時点でコードとして鳴る。
///
/// オーバーレイは空の 1 行で開くので、行を移る契機が無いまま最初のコードを打つ。
/// ここで鳴らないと「コードを認識していない」ようにしか見えない。
#[test]
fn typing_a_chord_name_sounds_the_chord_right_away() {
    let mut overlay = opened();
    let now = Instant::now();

    let action = overlay.handle_key(press(KeyCode::Char('C')), now);

    let MmlOverlayAction::Send(messages) = &action else {
        panic!("expected note on messages, got {action:?}");
    };
    let pitches = messages
        .iter()
        .filter(|message| message[0] == NOTE_ON)
        .map(|message| message[1])
        .collect::<Vec<_>>();
    assert_eq!(pitches, vec![60, 64, 67]);
    assert_eq!(overlay.sounding(), [60, 64, 67]);
    assert!(overlay.sounding_from_chord());
}

/// コード表記の行はコードとして鳴らす。
#[test]
fn a_chord_line_is_played_as_a_chord() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "C", now);
    overlay.handle_key(press(KeyCode::Enter), now);

    let action = overlay.handle_key(press(KeyCode::Up), now);

    assert_eq!(played_pitches(&action), vec![60, 64, 67]);
    assert_eq!(
        overlay.line_status(),
        &LineStatus::Played {
            from_chord: true,
            note_count: 3,
        }
    );
}

/// 空行へ移ったら、前の行の演奏は止める（鳴らすものは無い）。
#[test]
fn moving_to_an_empty_line_stops_the_previous_line() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);

    let action = overlay.handle_key(press(KeyCode::Enter), now);

    assert_eq!(
        action,
        MmlOverlayAction::PlayLine {
            patch: PatchChange::Keep,
            events: Vec::new(),
        }
    );
    assert_eq!(overlay.line_status(), &LineStatus::Idle);
}

#[test]
fn ctrl_space_replays_the_current_line() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);

    let action = overlay.handle_key(ctrl(KeyCode::Char(' ')), now);

    assert_eq!(played_pitches(&action), vec![60, 62, 64]);
}

/// 行を鳴らすと音源ごと止まるので、打鍵で鳴っていた音の記録は落とす。
/// 残すと、同じ行へ戻ったときにその音が鳴り直さない。
#[test]
fn playing_a_line_forgets_the_typed_note() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);
    assert_eq!(overlay.sounding(), [60]);

    overlay.handle_key(ctrl(KeyCode::Char(' ')), now);

    assert!(overlay.sounding().is_empty());
    // 同じ発音単位の中へ戻っただけでも、記録を落としてあるので鳴り直す。
    assert_eq!(
        overlay.handle_key(press(KeyCode::Left), now),
        MmlOverlayAction::Send(vec![[0x90, 60, 127]])
    );
}

#[test]
fn a_line_that_cannot_be_parsed_reports_the_error_and_stays_silent() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "r", now);

    let action = overlay.handle_key(ctrl(KeyCode::Char(' ')), now);

    assert_eq!(
        action,
        MmlOverlayAction::PlayLine {
            patch: PatchChange::Keep,
            events: Vec::new(),
        }
    );
    assert!(matches!(overlay.line_status(), LineStatus::Error(_)));
}
