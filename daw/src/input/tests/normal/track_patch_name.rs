use super::*;

#[test]
fn current_track_patch_name_uses_current_track_init_measure() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.data[2][0] = r#"{"patch":"Pads/Other.fxp:"}"#.to_string();
    app.editor.data[3][0] = r#"{"patch":"Keys/Current.fxp:"}"#.to_string();
    app.editor.cursor_track = 3;

    assert_eq!(
        app.current_track_patch_name().as_deref(),
        Some("Keys/Current.fxp")
    );
}

#[test]
fn current_track_patch_name_uses_init_saw_without_valid_patch() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.data[2][0] = r#"{"patch":":"}"#.to_string();

    assert_eq!(app.current_track_patch_name(), None);

    app.editor.data[2][0] = "{invalid".to_string();
    assert_eq!(app.current_track_patch_name(), None);
}

/// 旧キー・タグの無い旧形式は音色として読まない（既定音色の扱いにはせず、鳴らす側の server がエラーにする）。
#[test]
fn current_track_patch_name_ignores_the_legacy_key_and_untagged_values() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    for legacy in [
        r#"{"Surge XT patch":"Pads/Pad 1.fxp:"}"#,
        r#"{"patch":"Pads/Pad 1.fxp"}"#,
    ] {
        app.editor.data[2][0] = legacy.to_string();
        assert_eq!(app.current_track_patch_name(), None, "{legacy}");
    }
}

/// init セルへは wire（空タグの `:` 付き）で書き、読み返すと表示名に戻る。
/// favorites・履歴のキーは表示名なので、既存のキーと一致し続ける。
#[test]
fn the_init_cell_stores_the_wire_and_reads_back_the_display_name() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.data[2][0] = DawApp::build_patch_json("01 Basses/Abgrund.h2p");

    assert_eq!(
        app.editor.data[2][0],
        r#"{"patch":"01 Basses/Abgrund.h2p:"}"#
    );
    assert_eq!(
        app.current_track_patch_name().as_deref(),
        Some("01 Basses/Abgrund.h2p")
    );
}

/// Zebralette3 の表示名は `:z` を残した形で、init セルにもそのまま `.h2p:z` で書かれる。
#[test]
fn a_zebralette3_display_name_is_written_as_its_tagged_wire() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.data[2][0] = DawApp::build_patch_json("1 Poly Keys/Applicant Replicant.h2p:z");

    assert_eq!(
        app.editor.data[2][0],
        r#"{"patch":"1 Poly Keys/Applicant Replicant.h2p:z"}"#
    );
    assert_eq!(
        app.current_track_patch_name().as_deref(),
        Some("1 Poly Keys/Applicant Replicant.h2p:z")
    );
}
