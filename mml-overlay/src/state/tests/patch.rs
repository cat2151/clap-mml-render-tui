//! 行頭 JSON への音色 insert / overwrite と、音色選択の試聴。

use super::*;

const LEAD_JSON: &str = r#"{"Surge XT patch": "Leads/Lead 1.fxp"}"#;
const PAD_JSON: &str = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#;

fn patches() -> Vec<(String, String)> {
    ["Leads/Lead 1.fxp", "Pads/Pad 1.fxp"]
        .into_iter()
        .map(|patch| (patch.to_string(), patch.to_lowercase()))
        .collect()
}

fn opened_with_patches() -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(patches());
    overlay
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn column(overlay: &MmlOverlay<'_>) -> usize {
    let DataCursor(_, column) = overlay.textarea().cursor();
    column
}

#[test]
fn ctrl_t_opens_the_patch_select() {
    let mut overlay = opened_with_patches();

    overlay.handle_key(ctrl(KeyCode::Char('t')), Instant::now());

    assert!(overlay.patch_select().is_some());
}

#[test]
fn ctrl_t_does_nothing_while_the_patch_list_is_still_loading() {
    let mut overlay = opened();

    overlay.handle_key(ctrl(KeyCode::Char('t')), Instant::now());

    assert!(overlay.patch_select().is_none());
}

#[test]
fn moving_in_the_patch_select_previews_the_patch_with_the_note_at_the_cursor() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    for code in "cde".chars().map(KeyCode::Char) {
        overlay.handle_key(press(code), now);
    }
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Down), now),
        MmlOverlayAction::SetPatch {
            patch: Some("Pads/Pad 1.fxp".to_string()),
            messages: vec![[0x80, 64, 0], [0x90, 64, 127]],
        }
    );
}

#[test]
fn previewing_an_empty_mml_sounds_the_fallback_note() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Down), now),
        MmlOverlayAction::SetPatch {
            patch: Some("Pads/Pad 1.fxp".to_string()),
            messages: vec![[0x90, PREVIEW_PITCH, PREVIEW_VELOCITY]],
        }
    );
}

#[test]
fn confirming_inserts_the_patch_json_and_keeps_the_cursor_on_the_same_note() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    for code in "cde".chars().map(KeyCode::Char) {
        overlay.handle_key(press(code), now);
    }
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Enter), now);

    assert_eq!(overlay.value(), format!("{LEAD_JSON} cde"));
    assert_eq!(column(&overlay), LEAD_JSON.chars().count() + 1 + 3);
    assert_eq!(overlay.patch(), Some("Leads/Lead 1.fxp"));
    assert!(overlay.patch_select().is_none());
}

#[test]
fn confirming_again_overwrites_the_existing_patch_json() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    for code in "cde".chars().map(KeyCode::Char) {
        overlay.handle_key(press(code), now);
    }
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Enter), now);

    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Down), now);
    overlay.handle_key(press(KeyCode::Enter), now);

    assert_eq!(overlay.value(), format!("{PAD_JSON} cde"));
    assert_eq!(column(&overlay), PAD_JSON.chars().count() + 1 + 3);
}

#[test]
fn cancelling_restores_the_patch_that_was_current_when_it_opened() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Enter), now);

    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Down), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Esc), now),
        MmlOverlayAction::SetPatch {
            patch: Some("Leads/Lead 1.fxp".to_string()),
            messages: Vec::new(),
        }
    );
    assert_eq!(overlay.value(), format!("{LEAD_JSON} "));
    assert!(overlay.is_open());
}

#[test]
fn cancelling_without_previewing_asks_for_nothing() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Esc), now),
        MmlOverlayAction::Continue
    );
    assert!(overlay.patch_select().is_none());
}

#[test]
fn reopening_restores_the_patch_but_not_the_mml() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    for code in "cde".chars().map(KeyCode::Char) {
        overlay.handle_key(press(code), now);
    }
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Enter), now);
    overlay.handle_key(press(KeyCode::Esc), now);

    overlay.open(patches());

    assert_eq!(overlay.value(), format!("{LEAD_JSON} "));
    assert_eq!(overlay.patch(), Some("Leads/Lead 1.fxp"));
}

#[test]
fn a_hand_edited_patch_json_is_what_gets_remembered() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    for ch in format!("{PAD_JSON} c").chars() {
        overlay.handle_key(press(KeyCode::Char(ch)), now);
    }
    overlay.handle_key(press(KeyCode::Esc), now);

    assert_eq!(overlay.patch(), Some("Pads/Pad 1.fxp"));
}

#[test]
fn a_cursor_inside_the_patch_json_sounds_nothing() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Enter), now);

    // 行頭へ戻ってから1つ進める。ここは JSON の中なので鳴らない。
    overlay.handle_key(ctrl(KeyCode::Char('a')), now);
    assert_eq!(
        overlay.handle_key(press(KeyCode::Right), now),
        MmlOverlayAction::Continue
    );

    // JSON を抜けた最初の音でまた鳴る（試聴で鳴っていた音を止めてから）。
    for _ in 0..LEAD_JSON.chars().count() {
        overlay.handle_key(press(KeyCode::Right), now);
    }
    assert_eq!(
        overlay.handle_key(press(KeyCode::Right), now),
        MmlOverlayAction::Send(vec![[0x80, 60, 0], [0x90, 60, 127]])
    );
}
