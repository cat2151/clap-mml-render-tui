//! `Ctrl+O` のフレーズ履歴。

use super::*;

const PAD_JSON: &str = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#;

fn opened_with_history() -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        history: vec!["cdefg".to_string(), format!("{PAD_JSON} gfedc")],
        favorites: vec!["'ceg'".to_string()],
        ..MmlOverlayContext::default()
    });
    overlay
}

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
fn ctrl_o_opens_the_history() {
    let mut overlay = opened_with_history();

    overlay.handle_key(ctrl(KeyCode::Char('o')), Instant::now());

    assert!(overlay.history_select().is_some());
}

#[test]
fn ctrl_o_does_nothing_when_there_is_no_history() {
    let mut overlay = opened();

    overlay.handle_key(ctrl(KeyCode::Char('o')), Instant::now());

    assert!(overlay.history_select().is_none());
}

#[test]
fn moving_through_the_history_previews_the_selected_phrase() {
    let mut overlay = opened_with_history();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('o')), now);

    let action = overlay.handle_key(press(KeyCode::Down), now);

    assert_eq!(played_pitches(&action), vec![67, 65, 64, 62, 60]);
}

/// 履歴の行に音色が書いてあれば、その音色で試聴する。
#[test]
fn a_phrase_with_a_patch_switches_the_patch_for_the_preview() {
    let mut overlay = opened_with_history();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('o')), now);

    let action = overlay.handle_key(press(KeyCode::Down), now);

    let MmlOverlayAction::PlayLine { patch, .. } = action else {
        panic!("expected a line playback");
    };
    assert_eq!(
        patch,
        PatchChange::Switch(Some("Pads/Pad 1.fxp".to_string()))
    );
}

/// 音色を持たない行は、いま選んでいる音色のまま鳴らす。
/// 既定音色へ戻すと、選んだ音色でフレーズを試せなくなる。
#[test]
fn a_phrase_without_a_patch_keeps_the_current_patch() {
    let mut overlay = opened_with_history();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('o')), now);
    overlay.handle_key(press(KeyCode::Down), now);

    let action = overlay.handle_key(press(KeyCode::Up), now);

    let MmlOverlayAction::PlayLine { patch, .. } = action else {
        panic!("expected a line playback");
    };
    assert_eq!(patch, PatchChange::Keep);
}

/// 確定すると入力欄はその 1 行になる。書いていたものは捨ててよい前提。
#[test]
fn confirming_replaces_the_whole_input() {
    let mut overlay = opened_with_history();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(press(KeyCode::Enter), now);
    type_chars(&mut overlay, "efg", now);
    overlay.handle_key(ctrl(KeyCode::Char('o')), now);
    overlay.handle_key(press(KeyCode::Enter), now);

    assert_eq!(overlay.value(), "cdefg");
    assert!(overlay.history_select().is_none());
}

/// 行頭 JSON はテキストへ持ち込まず、音色として取り込む。
#[test]
fn confirming_takes_the_patch_out_of_the_phrase() {
    let mut overlay = opened_with_history();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('o')), now);
    overlay.handle_key(press(KeyCode::Down), now);
    overlay.handle_key(press(KeyCode::Enter), now);

    assert_eq!(overlay.value(), "gfedc");
    assert_eq!(overlay.patch(), Some("Pads/Pad 1.fxp"));
}

/// 確定した行は、そのまま続けて編集できる（カーソルは行末）。
#[test]
fn the_confirmed_phrase_can_be_edited_right_away() {
    let mut overlay = opened_with_history();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('o')), now);
    overlay.handle_key(press(KeyCode::Enter), now);

    overlay.handle_key(press(KeyCode::Char('a')), now);

    assert_eq!(overlay.value(), "cdefga");
}

#[test]
fn tab_moves_between_history_and_favorites() {
    let mut overlay = opened_with_history();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('o')), now);

    let action = overlay.handle_key(press(KeyCode::Tab), now);

    assert_eq!(played_pitches(&action), vec![60, 64, 67]);
}

#[test]
fn cancelling_after_a_preview_stops_the_sound_and_restores_the_patch() {
    let mut overlay = opened_with_history();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('o')), now);
    overlay.handle_key(press(KeyCode::Down), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Esc), now),
        MmlOverlayAction::PlayLine {
            patch: PatchChange::Switch(None),
            events: Vec::new(),
        }
    );
    assert!(overlay.history_select().is_none());
    assert!(overlay.is_open());
}

#[test]
fn cancelling_without_previewing_asks_for_nothing() {
    let mut overlay = opened_with_history();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('o')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Esc), now),
        MmlOverlayAction::Continue
    );
    assert!(overlay.history_select().is_none());
}

/// 絞り込むと選択が変わり、その行が試聴される。
#[test]
fn typing_filters_the_history() {
    let mut overlay = opened_with_history();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('o')), now);

    // "g" だけでは両方の行が残り、選択も動かない。"gf" で初めて絞り込まれる。
    let action = type_chars(&mut overlay, "gf", now);

    assert_eq!(played_pitches(&action), vec![67, 65, 64, 62, 60]);
}
