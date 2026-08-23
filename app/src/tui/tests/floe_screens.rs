//! Floe preset が共有 patch list と MML overlay をそのまま通ることの番人。

use std::time::Instant;

use super::*;
use cmrt_mml_overlay::{MmlOverlay, MmlOverlayAction, MmlOverlayContext};

const FLOE_PATCH: &str = "Celtic Harp Factory Presets/Realistic Celtic Harp.floe-preset";

fn app_with_floe_patch() -> TuiApp<'static> {
    let app = TuiApp::new_for_test(test_config());
    *app.patch_load_state.lock().unwrap() = PatchLoadState::Ready(make_patches(&[
        "patches_factory/Pads/Pad 1.fxp",
        FLOE_PATCH,
    ]));
    app
}

#[test]
fn mml_overlay_receives_floe_from_the_shared_patch_list() {
    let app = app_with_floe_patch();

    assert!(app
        .loaded_patch_pairs()
        .iter()
        .any(|(display, _)| display == FLOE_PATCH));
    assert!(Arc::ptr_eq(
        &app.patch_load_state,
        &app.notepad.patch_load_state
    ));
}

#[test]
fn selecting_floe_requests_a_preview_and_keeps_the_display_string() {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        patches: make_patches(&["patches_factory/Pads/Pad 1.fxp", FLOE_PATCH]),
        ..MmlOverlayContext::default()
    });
    let now = Instant::now();
    overlay.handle_key(
        KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL),
        now,
    );

    let mut preview = None;
    for ch in "floe".chars() {
        let action = overlay.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE), now);
        if let MmlOverlayAction::SetPatch { patch, messages } = action {
            assert!(!messages.is_empty());
            preview = patch;
        }
    }
    assert_eq!(preview.as_deref(), Some(FLOE_PATCH));

    overlay.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), now);
    assert_eq!(overlay.patch(), Some(FLOE_PATCH));
}

#[test]
fn floe_display_string_survives_the_mml_head_json() {
    let mut app = app_with_floe_patch();
    app.notepad
        .set_session_lines_for_test(vec![format!("{{\"Surge XT patch\":\"{FLOE_PATCH}\"}} c")]);

    app.start_keyboard_from_notepad();

    assert_eq!(app.keyboard.state.patch(), Some(FLOE_PATCH));
}
