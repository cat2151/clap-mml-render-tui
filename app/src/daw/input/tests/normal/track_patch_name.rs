use super::*;

#[test]
fn current_track_patch_name_uses_current_track_init_measure() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.data[1][0] = r#"{"Surge XT patch":"Pads/Other.fxp"}"#.to_string();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Keys/Current.fxp"}"#.to_string();
    app.editor.cursor_track = 2;

    assert_eq!(
        app.current_track_patch_name().as_deref(),
        Some("Keys/Current.fxp")
    );
}

#[test]
fn current_track_patch_name_uses_init_saw_without_valid_patch() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.data[1][0] = r#"{"Surge XT patch":""}"#.to_string();

    assert_eq!(app.current_track_patch_name(), None);

    app.editor.data[1][0] = "{invalid".to_string();
    assert_eq!(app.current_track_patch_name(), None);
}
