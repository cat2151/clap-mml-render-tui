use super::*;

#[test]
fn start_patch_select_builds_favorite_items_in_registered_order() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Pad B", "Pad A",
    ]))));
    app.patch_phrase_store.favorite_patches = vec![
        "Pad 2".to_string(),
        "Pad A".to_string(),
        "Pad 11".to_string(),
    ];
    app.patch_phrase_store.patches.insert(
        "Pad A".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec![],
            favorites: vec!["l8cdef".to_string()],
        },
    );
    app.patch_phrase_store.patches.insert(
        "Pad 11".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec![],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.patch_phrase_store.patches.insert(
        "Pad 2".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec![],
            favorites: vec!["o4c".to_string()],
        },
    );

    app.start_patch_select();

    assert_eq!(
        app.patch_select.patch_favorite_items,
        vec![
            "Pad 2".to_string(),
            "Pad A".to_string(),
            "Pad 11".to_string()
        ]
    );
}

#[test]
fn start_patch_select_migrates_prefixed_favorites_from_legacy_patch_name() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "patches_factory/Pads/Pad 1.fxp",
    ]))));
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec!["hist".to_string()],
            favorites: vec!["fav".to_string()],
        },
    );

    app.start_patch_select();

    assert_eq!(
        app.patch_select.patch_favorite_items,
        vec!["patches_factory/Pads/Pad 1.fxp".to_string()]
    );
    assert!(app
        .patch_phrase_store
        .patches
        .contains_key("patches_factory/Pads/Pad 1.fxp"));
    assert!(!app
        .patch_phrase_store
        .patches
        .contains_key("Pads/Pad 1.fxp"));
}

#[test]
fn open_patch_select_overlay_selects_requested_initial_patch() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Pads/Pad 1.fxp",
        "Leads/Lead 1.fxp",
        "Bass/Bass 1.fxp",
    ]))));

    app.open_patch_select_overlay(Some("Leads/Lead 1.fxp"));

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(
        app.patch_select.patch_filtered,
        vec![
            "Pads/Pad 1.fxp".to_string(),
            "Leads/Lead 1.fxp".to_string(),
            "Bass/Bass 1.fxp".to_string()
        ]
    );
    assert_eq!(app.patch_select.patch_cursor, 1);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(1));
    assert_eq!(
        app.patch_select.patch_select_focus,
        PatchSelectPane::Patches
    );
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Leads/Lead 1.fxp"} l8cdef"#
    ));
    let cache = app.audio.cache.lock().unwrap();
    assert!(cache.contains_key(r#"{"Surge XT patch": "Pads/Pad 1.fxp"} l8cdef"#));
    assert!(cache.contains_key(r#"{"Surge XT patch": "Bass/Bass 1.fxp"} l8cdef"#));
}
