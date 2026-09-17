//! chord 行の `r` = コード進行の抽選と、小節数を進行にそろえる挙動のテスト。
//!
//! 進行の綴りの期待値（`vi` → `VIm`）の出どころは [`super::chord_wizard`] の module doc。

use super::*;

use super::chord_wizard::{build_wide_test_app, chord_row, last_log, set_catalog, MEASURE, TRACK};
use crate::CHORD_TRACK;

fn press_r(app: &mut DawApp) {
    app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
}

fn cursor_on_chord_row(app: &mut DawApp) {
    app.editor.cursor_track = CHORD_TRACK;
    app.editor.cursor_measure = MEASURE;
}

fn chords(degrees: &[&str]) -> Vec<String> {
    degrees.iter().map(|s| (*s).to_string()).collect()
}

/// 2 小節の grid に 4 コードの進行を引くと、grid が 4 小節へ広がって全部入る。
#[test]
fn pressing_r_on_the_chord_row_writes_the_progression_and_grows_the_grid_to_fit() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    cursor_on_chord_row(&mut app);
    set_catalog(&mut app, &["I-V-vi-IV"]);

    press_r(&mut app);

    assert_eq!(app.editor.measures, 4);
    assert_eq!(chord_row(&app), ["I", "V", "VIm", "IV"]);
    assert_eq!(app.editor.cursor_measure, MEASURE);
    // 演奏 track の init は触らない（どの track が鳴らすかは決めない）。
    assert_eq!(app.editor.data[TRACK][0], "");
}

/// 8 小節の grid に 2 コードの進行を引くと、grid が 2 小節へ縮む。
#[test]
fn a_shorter_progression_shrinks_the_grid_to_its_length() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_wide_test_app();
    cursor_on_chord_row(&mut app);
    app.editor.cursor_measure = 6;
    app.editor.data[CHORD_TRACK][5] = "V".to_string();

    app.apply_random_chord_progression_with(&chords(&["I", "IV"]));

    assert_eq!(app.editor.measures, 2);
    assert_eq!(chord_row(&app), ["I", "IV"]);
    assert_eq!(app.editor.cursor_measure, MEASURE);
    assert_eq!(app.playback.measure_mmls.lock().unwrap().len(), 2);
}

/// 減らして落ちる演奏 track のセルは失われる。黙って消さず数をログに出す。
#[test]
fn shrinking_logs_how_many_playable_cells_were_dropped() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_wide_test_app();
    cursor_on_chord_row(&mut app);
    app.editor.data[TRACK][5] = "cde".to_string();
    app.editor.data[TRACK][7] = "efg".to_string();

    app.apply_random_chord_progression_with(&chords(&["I", "IV"]));

    assert_eq!(app.editor.data[TRACK].len(), 3);
    assert!(app.log_lines.lock().unwrap().iter().any(|line| {
        line == "random chord: 小節を減らしたため、演奏 track の 2 セルを捨てました"
    }));
}

#[test]
fn an_empty_catalog_logs_instead_of_resizing() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    cursor_on_chord_row(&mut app);

    press_r(&mut app);

    assert_eq!(app.editor.measures, 2);
    assert_eq!(app.editor.data[CHORD_TRACK][MEASURE], "");
    assert_eq!(
        last_log(&app).as_deref(),
        Some("コード進行カタログが空です")
    );
}

/// 演奏 track の `r` は従来どおりランダム音色で、コード進行には触らない。
#[test]
fn r_on_a_playable_track_still_leaves_the_chord_row_alone() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = TRACK;
    app.editor.cursor_measure = MEASURE;
    set_catalog(&mut app, &["I-V-vi-IV"]);

    press_r(&mut app);

    assert_eq!(app.editor.measures, 2);
    assert_eq!(chord_row(&app), ["", ""]);
}
