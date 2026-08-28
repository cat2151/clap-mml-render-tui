//! 1 行モードの確定（`Enter` / `Esc`）の検証。
//!
//! 手触りは従来のインライン INSERT と同じ。`Enter` はセルへ書き戻して次の meas の
//! 入力欄を開き、`Esc` は書き戻して閉じる。

use crossterm::event::KeyCode;

use super::super::super::{DawApp, DawMode};
use super::{ctrl, key, plain};
use crate::input::tests::build_test_app;

/// track1 の meas1・meas2 に MML を置いて、meas1 のセルを開いた状態にする。
fn opened_on_the_first_measure() -> (DawApp, std::sync::mpsc::Receiver<crate::CacheJob>) {
    let (mut app, cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    app.editor.data[1][1] = "cde".to_string();
    app.editor.data[1][2] = "gab".to_string();
    assert!(app.try_open_mml_overlay(ctrl('p')));
    (app, cache_rx)
}

#[test]
fn enter_writes_the_line_back_and_opens_the_next_measure() {
    let (mut app, _cache_rx) = opened_on_the_first_measure();

    // 1 行モードの入力欄はカーソルが行末にある（Stage 5）。
    app.handle_mml_overlay_key_event(plain('f'));
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));

    assert_eq!(app.editor.data[1][1], "cdef", "確定した行がセルへ入るはず");
    assert_eq!(app.editor.cursor_measure, 2, "次の meas へ進むはず");
    assert_eq!(app.mode, DawMode::MmlOverlay);
    assert!(
        app.mml_overlay.is_open(),
        "続けて書けるよう開いたままにする"
    );
    assert_eq!(
        app.mml_overlay.value(),
        "gab",
        "開き直した入力欄には次のセルの MML が入るはず"
    );
}

#[test]
fn enter_on_the_last_measure_keeps_the_cursor_in_range() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = app.editor.measures;
    app.editor.data[1][app.editor.measures] = "gab".to_string();
    assert!(app.try_open_mml_overlay(ctrl('p')));

    app.handle_mml_overlay_key_event(plain('c'));
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));

    assert_eq!(app.editor.cursor_measure, app.editor.measures);
    assert_eq!(app.editor.data[1][app.editor.measures], "gabc");
    assert!(app.mml_overlay.is_open());
    assert_eq!(
        app.mml_overlay.value(),
        "gabc",
        "最終 meas ではそのセルのまま開き直す"
    );
}

#[test]
fn esc_writes_the_line_back_and_closes() {
    let (mut app, _cache_rx) = opened_on_the_first_measure();

    app.handle_mml_overlay_key_event(plain('f'));
    app.handle_mml_overlay_key_event(key(KeyCode::Esc));

    assert_eq!(app.editor.data[1][1], "cdef");
    assert_eq!(app.editor.cursor_measure, 1, "閉じるときは進めない");
    assert_eq!(app.mode, DawMode::Normal);
    assert!(!app.mml_overlay.is_open());
}

#[test]
fn an_emptied_line_clears_the_cell() {
    let (mut app, _cache_rx) = opened_on_the_first_measure();

    for _ in 0.."cde".len() {
        app.handle_mml_overlay_key_event(key(KeyCode::Backspace));
    }
    app.handle_mml_overlay_key_event(key(KeyCode::Esc));

    assert_eq!(app.editor.data[1][1], "");
}

#[test]
fn the_daw_keys_still_work_after_committing_with_esc() {
    let (mut app, _cache_rx) = opened_on_the_first_measure();
    app.handle_mml_overlay_key_event(key(KeyCode::Esc));

    app.handle_normal_key_event(plain('h'));

    assert_eq!(app.editor.cursor_measure, 0);
}

/// 実機の config.toml を渡したときだけ走る、実カタログでの `Enter` 確定の所要時間。
///
/// 確定のたびに次の meas の入力欄を開き直す＝音色一覧のスナップショットを毎回
/// オーバーレイへ渡し直すので、カタログが実サイズ（5000 件規模）でも
/// 入力の手が止まらないことを確かめる。開発機のインストール状況に依存するため、
/// 環境変数が無ければ skip する（個人のパスはコードへ書かない）。
///
/// ```text
/// CMRT_TEST_DAW_REAL_CONFIG=%LOCALAPPDATA%\clap-mml-render-tui\config.toml \
///   cargo test -p cmrt-daw --lib real_catalog -- --nocapture
/// ```
#[test]
fn real_catalog_commit_opens_the_next_measure_without_stalling() {
    let Some(config_path) = std::env::var_os("CMRT_TEST_DAW_REAL_CONFIG") else {
        eprintln!("skip: CMRT_TEST_DAW_REAL_CONFIG が未設定");
        return;
    };
    let cfg = cmrt_runtime::Config::load_from_path(std::path::Path::new(&config_path))
        .expect("CMRT_TEST_DAW_REAL_CONFIG の config.toml を読めること");
    let pairs = cmrt_tui_core::patches::collect_patch_pairs(&cfg).expect("実カタログの走査");
    assert!(!pairs.is_empty(), "実機カタログが 0 件では比較にならない");

    let tmp = std::env::temp_dir().join("cmrt_test_daw_mml_overlay_real_catalog_commit");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();
    let (open_elapsed, commit_elapsed) = {
        let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);

        let (mut app, _cache_rx) = build_test_app();
        app.cfg = std::sync::Arc::new(cfg);
        *app.patch_load.lock().unwrap() =
            cmrt_tui_core::patch_load::PatchLoadState::ready(pairs.clone());
        app.editor.cursor_track = 1;
        app.editor.cursor_measure = 1;
        app.editor.data[1][1] = "cde".to_string();
        app.editor.data[1][2] = "gab".to_string();

        let started = std::time::Instant::now();
        assert!(app.try_open_mml_overlay(ctrl('p')));
        let open_elapsed = started.elapsed();

        let started = std::time::Instant::now();
        app.handle_mml_overlay_key_event(key(KeyCode::Enter));
        let commit_elapsed = started.elapsed();

        assert_eq!(app.editor.cursor_measure, 2);
        assert_eq!(app.mml_overlay.value(), "gab");
        (open_elapsed, commit_elapsed)
    };
    std::fs::remove_dir_all(&tmp).ok();

    eprintln!(
        "real catalog: count={} open(ctrl+p)={open_elapsed:?} commit(enter)={commit_elapsed:?}",
        pairs.len()
    );
    assert!(
        commit_elapsed < std::time::Duration::from_millis(200),
        "確定が引っかかっている: {commit_elapsed:?}"
    );
}
