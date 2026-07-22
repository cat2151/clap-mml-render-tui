use super::*;

#[test]
fn handle_normal_enter_rewrites_legacy_patch_json_with_prefixed_patch_name() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "patches_factory/Pads/Pad 1.fxp",
    ]))));

    app.handle_normal(KeyCode::Enter);

    assert_eq!(
        app.editor.lines,
        vec![r#"{"Surge XT patch": "patches_factory/Pads/Pad 1.fxp"} l8cdef"#.to_string()]
    );
    assert!(matches!(
        &*app.playback.play_state.lock().unwrap(),
        PlayState::Running(msg)
            if msg == r#"{"Surge XT patch": "patches_factory/Pads/Pad 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_normal_enter_keeps_saved_patch_filter_when_normalizing_patch_name() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![
        r#"{"Surge XT patch":"Pads/Pad 1.fxp","Surge XT patch filter":"pads"} l8cdef"#.to_string(),
    ];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "patches_factory/Pads/Pad 1.fxp",
    ]))));

    app.handle_normal(KeyCode::Enter);

    assert_eq!(
        app.editor.lines,
        vec![
            r#"{"Surge XT patch": "patches_factory/Pads/Pad 1.fxp", "Surge XT patch filter": "pads"} l8cdef"#.to_string()
        ]
    );
}
