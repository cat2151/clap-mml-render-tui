//! `G` = chord wizard の**入口そのもの**（キー `G` を押したときの挙動）のテスト。
//!
//! 「配られた結果が正しいか」は [`super::chord_wizard`] が持つ。こちらが持つのは
//! **押した瞬間に何が決まるか**の 3 つ:
//!
//! - カタログから進行と音色をどう抽選するか
//! - 押してはいけない場所（init 列 / chord 行）で押したときに**何もせずログだけ**返すこと
//! - `dd` の待ち状態を持ち越さないこと
//!
//! 生成 MML の期待値の出どころは [`super::chord_wizard`] の module doc にある。

use super::*;

use super::chord_wizard::{
    build_wide_test_app, chord_row, cursor_at_wizard_target, last_log, press_g, set_catalog,
    MEASURE, TRACK,
};
use crate::CHORD_TRACK;

#[test]
fn pressing_g_picks_from_the_injected_catalog() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_wide_test_app();
    cursor_at_wizard_target(&mut app);
    // 音色が既にあると、この経路では音色の抽選（= 実 file の走査）が走らない。
    app.editor.data[TRACK][0] = r#"{"Surge XT patch": "Chosen.fxp"}"#.to_string();
    set_catalog(&mut app, &["I-V-vi-IV"]);

    press_g(&mut app);

    assert_eq!(chord_row(&app), ["I", "V", "VIm", "IV", "", "", "", ""]);
    assert_eq!(
        app.editor.data[TRACK][0],
        r#"{"Surge XT patch":"Chosen.fxp","generate from chord track":"close"}"#
    );
}

/// 音色未選択の track に wizard が音色を補うときは、共通 PatchRole の Chord 候補だけを使う。
#[test]
fn pressing_g_picks_only_a_chord_role_patch() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_wide_test_app();
    cursor_at_wizard_target(&mut app);
    set_catalog(&mut app, &["I-IV"]);
    *app.patch_load.lock().unwrap() = cmrt_tui_core::patch_load::PatchLoadState::ready(vec![
        (
            "Bass/Deep Bass.fxp".to_string(),
            "bass/deep bass.fxp".to_string(),
        ),
        (
            "Pads/Warm Pad.fxp".to_string(),
            "pads/warm pad.fxp".to_string(),
        ),
        (
            "Leads/Bright Lead.fxp".to_string(),
            "leads/bright lead.fxp".to_string(),
        ),
    ]);

    press_g(&mut app);

    assert_eq!(
        app.current_track_patch_name().as_deref(),
        Some("Pads/Warm Pad.fxp")
    );
}

/// 抽選したものが chord2mml を通ることまで確かめてから書く。
/// 通らないものしか無いカタログでは、1 セルも書かずにログだけ残す。
#[test]
fn a_progression_that_chord2mml_rejects_is_never_written() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    cursor_at_wizard_target(&mut app);
    set_catalog(&mut app, &["???"]);

    press_g(&mut app);

    assert_eq!(app.editor.data[CHORD_TRACK][MEASURE], "");
    assert_eq!(app.editor.data[TRACK][0], "");
    assert_eq!(
        last_log(&app).as_deref(),
        Some("コード進行を 16 回引きましたが、鳴るものがありませんでした")
    );
}

#[test]
fn an_empty_catalog_logs_instead_of_writing() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    cursor_at_wizard_target(&mut app);

    press_g(&mut app);

    assert_eq!(app.editor.data[CHORD_TRACK][MEASURE], "");
    assert_eq!(
        last_log(&app).as_deref(),
        Some("コード進行カタログが空です")
    );
}

#[test]
fn the_wizard_rejects_the_init_column() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    cursor_at_wizard_target(&mut app);
    app.editor.cursor_measure = 0;
    set_catalog(&mut app, &["I-IV"]);

    press_g(&mut app);

    assert_eq!(
        last_log(&app).as_deref(),
        Some("chord wizard は init 以外の小節でのみ使用できます")
    );
}

/// chord 行の上で `G` を押しても、chord 行自身は生成対象にならない。
#[test]
fn the_wizard_rejects_the_chord_row_itself() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    cursor_at_wizard_target(&mut app);
    app.editor.cursor_track = CHORD_TRACK;
    set_catalog(&mut app, &["I-IV"]);

    press_g(&mut app);

    assert_eq!(app.editor.data[CHORD_TRACK][MEASURE], "");
    assert_eq!(
        last_log(&app).as_deref(),
        Some("chord wizard は演奏トラックでのみ使用できます")
    );
}

/// `dd` の待ち状態を `G` が持ち越さない（`d` の次の `G` で誤って cut しない）。
#[test]
fn g_does_not_leave_a_pending_delete_armed() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    cursor_at_wizard_target(&mut app);
    app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

    press_g(&mut app);

    assert!(!app.editor.pending_delete);
}
