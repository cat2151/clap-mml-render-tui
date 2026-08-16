//! 音色は入力欄と別に持つ（`Ctrl+T` だけが書き換える）。

use super::*;

fn patches() -> Vec<(String, String)> {
    ["Leads/Lead 1.fxp", "Pads/Pad 1.fxp"]
        .into_iter()
        .map(|patch| (patch.to_string(), patch.to_lowercase()))
        .collect()
}

fn opened_with_patches() -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        patches: patches(),
        ..MmlOverlayContext::default()
    });
    overlay
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
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Down), now),
        MmlOverlayAction::SetPatch {
            patch: Some("Pads/Pad 1.fxp".to_string()),
            messages: vec![[0x90, 64, 127]],
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
            // 試聴用の `c` を既定のオクターブ・velocity で鳴らす。
            messages: vec![[0x90, 60, 127]],
        }
    );
}

/// 音色は入力欄には現れない。フレーズを 1 行ずつ書き並べる邪魔をしないため。
#[test]
fn confirming_keeps_the_input_untouched() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Enter), now);

    assert_eq!(overlay.value(), "cde");
    assert_eq!(overlay.patch(), Some("Leads/Lead 1.fxp"));
    assert!(overlay.patch_select().is_none());
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
    assert_eq!(overlay.patch(), Some("Leads/Lead 1.fxp"));
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
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Enter), now);
    overlay.handle_key(press(KeyCode::Esc), now);

    overlay.open(MmlOverlayContext {
        patches: patches(),
        ..MmlOverlayContext::default()
    });

    assert_eq!(overlay.value(), "");
    assert_eq!(overlay.patch(), Some("Leads/Lead 1.fxp"));
}
