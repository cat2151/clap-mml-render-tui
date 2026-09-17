use crate::input::tests::{build_test_app, temp_local_dirs};
use crate::{AbRepeatState, CHORD_TRACK, FIRST_PLAYABLE_TRACK};

fn column_counts(app: &crate::DawApp) -> (Vec<usize>, Vec<usize>) {
    let data = app.editor.data.iter().map(Vec::len).collect();
    let cache = app.cache.lock().unwrap().iter().map(Vec::len).collect();
    (data, cache)
}

#[test]
fn growing_adds_columns_to_every_row_and_the_cache() {
    let (_temp, _env_guard) = temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    let tracks = app.editor.tracks;

    let dropped = app.set_measure_count(6);

    assert_eq!(dropped, 0);
    assert_eq!(app.editor.measures, 6);
    assert_eq!(column_counts(&app), (vec![7; tracks], vec![7; tracks]));
}

/// drum のように全小節おなじ手書きの track が、増えた小節だけ無音にならないように、
/// 演奏 track の新しい小節には最後の小節を写す。写した小節は render キューへ入る。
#[test]
fn growing_copies_the_last_measure_of_each_playable_track_into_the_new_ones() {
    let (_temp, _env_guard) = temp_local_dirs("daw_cache");
    let (mut app, cache_rx) = build_test_app();
    app.editor.data[FIRST_PLAYABLE_TRACK][0] = r#"{"Surge XT patch": "Kick.fxp"}"#.to_string();
    app.editor.data[FIRST_PLAYABLE_TRACK][1] = "c8c8".to_string();
    app.editor.data[FIRST_PLAYABLE_TRACK][2] = "c4".to_string();

    app.set_measure_count(4);

    assert_eq!(
        app.editor.data[FIRST_PLAYABLE_TRACK][1..],
        ["c8c8", "c4", "c4", "c4"]
    );
    let kicked: Vec<(usize, usize)> = cache_rx
        .try_iter()
        .map(|job| (job.track, job.measure))
        .collect();
    assert_eq!(
        kicked,
        [(FIRST_PLAYABLE_TRACK, 3), (FIRST_PLAYABLE_TRACK, 4)]
    );
}

/// chord 行と tempo 行には写さない。chord 行は呼び出し側が書き直し、tempo 行の
/// 小節セルは前の小節の指定がそのまま効き続ける。
#[test]
fn growing_leaves_the_new_chord_and_tempo_cells_empty() {
    let (_temp, _env_guard) = temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    app.editor.data[CHORD_TRACK][2] = "IV".to_string();
    app.editor.data[0][2] = "t140".to_string();

    app.set_measure_count(3);

    assert_eq!(app.editor.data[CHORD_TRACK][3], "");
    assert_eq!(app.editor.data[0][3], "");
}

#[test]
fn shrinking_truncates_rows_and_reports_dropped_playable_cells() {
    let (_temp, _env_guard) = temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    let tracks = app.editor.tracks;
    app.editor.data[FIRST_PLAYABLE_TRACK][2] = "cde".to_string();
    app.editor.data[FIRST_PLAYABLE_TRACK + 1][2] = "efg".to_string();
    // chord 行は呼び出し側が書き直すので数えない。
    app.editor.data[CHORD_TRACK][2] = "IV".to_string();

    let dropped = app.set_measure_count(1);

    assert_eq!(dropped, 2);
    assert_eq!(app.editor.measures, 1);
    assert_eq!(column_counts(&app), (vec![2; tracks], vec![2; tracks]));
}

/// 小節 index を持つ状態は、減らしたあとに範囲外を指さないように片付ける。
#[test]
fn shrinking_clamps_the_cursor_and_forgets_measure_indexed_state() {
    let (_temp, _env_guard) = temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_measure = 2;
    app.editor.cell_undo = Some(vec![]);
    *app.playback.ab_repeat.lock().unwrap() = AbRepeatState::FixStart {
        start_measure_index: 1,
        end_measure_index: 1,
    };

    app.set_measure_count(1);

    assert_eq!(app.editor.cursor_measure, 1);
    assert_eq!(app.editor.cell_undo, None);
    assert_eq!(app.ab_repeat_state(), AbRepeatState::Off);
}

#[test]
fn the_same_count_is_a_no_op() {
    let (_temp, _env_guard) = temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_measure = 2;
    app.editor.cell_undo = Some(vec![]);

    let dropped = app.set_measure_count(app.editor.measures);

    assert_eq!(dropped, 0);
    assert_eq!(app.editor.cursor_measure, 2);
    assert_eq!(app.editor.cell_undo, Some(vec![]));
}
