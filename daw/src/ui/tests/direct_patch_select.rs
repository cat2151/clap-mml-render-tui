//! NORMAL の `t` は、MML 入力欄を開かずに音色 selector だけを grid の上へ重ねる。

use super::*;

use cmrt_tui_core::patch_load::PatchLoadState;

fn app_with_selector_open() -> DawApp {
    let mut app = build_test_app();
    *app.patch_load.lock().unwrap() = PatchLoadState::ready(
        ["Keys/Snapshot Keys.fxp", "Pads/Snapshot Pad.fxp"]
            .into_iter()
            .map(|display| (display.to_string(), display.to_lowercase()))
            .collect(),
    );
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] = r#"{"Surge XT patch": "Pads/Snapshot Pad.fxp"}"#.to_string();
    app.editor.data[2][1] = "cde".to_string();
    assert!(app.open_direct_patch_select());
    app
}

#[test]
fn t_draws_the_patch_selector_without_the_mml_input_box() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let app = app_with_selector_open();

    assert!(!app.mml_overlay.is_open());
    assert!(app
        .direct_patch_select
        .as_ref()
        .is_some_and(|select| select.is_select_open()));
    let screen = render_lines(&app, 120, 40).join("\n");
    assert!(
        screen.contains("Snapshot Keys"),
        "音色 selector が描かれているはず:\n{screen}"
    );
    // 入力欄の枠のタイトル（` MML [音色] `）が無いこと。
    assert!(
        !screen.contains("MML ["),
        "入力欄の枠が描かれていないはず:\n{screen}"
    );
}

#[test]
fn a_loading_catalog_shows_the_waiting_notice_without_the_mml_input_box() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let mut app = build_test_app();
    *app.patch_load.lock().unwrap() = PatchLoadState::Loading;
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    assert!(app.open_direct_patch_select());

    let screen = render_lines(&app, 120, 40).join("\n");

    assert!(!app.mml_overlay.is_open());
    assert!(!screen.contains("MML ["), "{screen}");
    // 全角は buffer でセルが分かれるので、空白を除いて探す（見つからなければ panic）。
    find_text_ignoring_spaces(&render_buffer(&app, 120, 40), "音色一覧を読み込み中");
}
