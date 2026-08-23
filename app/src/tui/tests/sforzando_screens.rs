//! Sforzando の `.sfz` が画面別の一覧を作らず共有 catalog を通ることの番人。

use std::time::Instant;

use super::*;
use cmrt_mml_overlay::{MmlOverlay, MmlOverlayAction, MmlOverlayContext, PatchCatalogSnapshot};

const SFZ_PATCH: &str = "Virtual-Playing-Orchestra3/Woodwinds/flute-SOLO-sustain.sfz";

fn app_with_sfz_patch() -> TuiApp<'static> {
    let app = TuiApp::new_for_test(test_config());
    *app.patch_load_state.lock().unwrap() = PatchLoadState::Ready(make_patches(&[
        "patches_factory/Pads/Pad 1.fxp",
        "Dexed_01.syx/00 Say Again.",
        SFZ_PATCH,
    ]));
    app
}

#[test]
fn every_screen_receives_sfz_from_the_one_shared_patch_list() {
    let app = app_with_sfz_patch();

    assert!(app
        .loaded_patch_pairs()
        .iter()
        .any(|(display, _)| display == SFZ_PATCH));
    assert!(Arc::ptr_eq(
        &app.patch_load_state,
        &app.notepad.patch_load_state
    ));
}

#[test]
fn selecting_sfz_requests_preview_without_changing_its_display_string() {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        patch_catalog: PatchCatalogSnapshot::Ready(make_patches(&[
            "patches_factory/Pads/Pad 1.fxp",
            SFZ_PATCH,
        ])),
        ..MmlOverlayContext::default()
    });
    let now = Instant::now();
    overlay.handle_key(
        KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL),
        now,
    );

    let mut preview = None;
    for ch in "flute".chars() {
        if let MmlOverlayAction::SetPatch { patch, notes } =
            overlay.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE), now)
        {
            assert!(notes.is_some_and(|notes| !notes.messages.is_empty()));
            preview = patch;
        }
    }
    assert_eq!(preview.as_deref(), Some(SFZ_PATCH));

    overlay.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), now);
    assert_eq!(overlay.patch(), Some(SFZ_PATCH));
}

#[test]
fn sfz_keeps_the_existing_mml_json_key_and_reaches_keyboard() {
    let mut app = app_with_sfz_patch();
    app.notepad
        .set_session_lines_for_test(vec![format!("{{\"Surge XT patch\":\"{SFZ_PATCH}\"}} c")]);

    app.start_keyboard_from_notepad();

    assert_eq!(app.keyboard.state.patch(), Some(SFZ_PATCH));
}
