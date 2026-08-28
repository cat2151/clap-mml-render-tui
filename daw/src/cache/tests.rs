use std::path::Path;

use super::workspace_cache_dir;
use crate::WorkspaceKind;

#[test]
fn workspace_cache_directory_keeps_persistent_path_and_nests_daily() {
    let plugin_namespace = Path::new("daw_cache").join("Surge XT");

    assert_eq!(
        workspace_cache_dir(&plugin_namespace, WorkspaceKind::Persistent),
        plugin_namespace
    );
    assert_eq!(
        workspace_cache_dir(&plugin_namespace, WorkspaceKind::Daily),
        plugin_namespace.join("daily")
    );
}

/// chord 行の中身はコード進行なので、レンダリングのジョブを作らない。
///
/// ここで止めないと chord2mml の文字列がそのまま MML パーサへ流れ、
/// 無音のセルを毎回レンダリングし続けることになる。
#[test]
fn the_chord_row_never_produces_a_cache_job() {
    let (mut app, cache_rx) = crate::input::tests::build_test_app();
    app.editor.data[crate::CHORD_TRACK][1] = "I-IV-V-I".to_string();
    app.editor.data[crate::FIRST_PLAYABLE_TRACK][1] = "cde".to_string();

    app.sync_cache_states();
    app.kick_cache(crate::CHORD_TRACK, 1);
    app.kick_cache(crate::FIRST_PLAYABLE_TRACK, 1);

    let jobs: Vec<usize> = std::iter::from_fn(|| cache_rx.try_recv().ok())
        .map(|job| job.track)
        .collect();

    assert_eq!(jobs, vec![crate::FIRST_PLAYABLE_TRACK]);
    let cache = app.cache.lock().unwrap();
    assert!(cache[crate::CHORD_TRACK][1].state == crate::CacheState::Empty);
}

/// 手書きセルが空でも、chord 行から生成されるセルはレンダリング対象になる。
///
/// 生のセル文字列だけで空判定していると、生成されたセルが丸ごと無視されて音が出ない。
#[test]
fn a_cell_generated_from_the_chord_row_is_queued_even_though_the_cell_itself_is_empty() {
    let (mut app, cache_rx) = crate::input::tests::build_test_app();
    app.editor.data[crate::CHORD_TRACK][1] = "I-IV".to_string();
    app.editor.data[crate::FIRST_PLAYABLE_TRACK][0] =
        r#"{"generate from chord track": "close"}"#.to_string();

    app.sync_cache_states();
    assert!(
        app.cache.lock().unwrap()[crate::FIRST_PLAYABLE_TRACK][1].state
            == crate::CacheState::Pending
    );

    app.kick_cache(crate::FIRST_PLAYABLE_TRACK, 1);

    let jobs: Vec<crate::CacheJob> = std::iter::from_fn(|| cache_rx.try_recv().ok()).collect();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].track, crate::FIRST_PLAYABLE_TRACK);
    // chord2mml.exe "close | I-IV |" の出力そのまま。
    assert!(
        jobs[0].mml.contains("v11/*|*/'c2eg''f2a<c'/*|*/"),
        "unexpected mml: {}",
        jobs[0].mml
    );
}

/// chord 行を編集したら、そこから生成されているセルのキャッシュが捨てられる。
///
/// 手書きのセルは chord 行に依存しないので巻き込まない。
#[test]
fn editing_the_chord_row_invalidates_only_the_cells_generated_from_it() {
    let (mut app, _cache_rx) = crate::input::tests::build_test_app();
    let generated = crate::FIRST_PLAYABLE_TRACK;
    let handwritten = crate::FIRST_PLAYABLE_TRACK + 1;
    app.editor.data[crate::CHORD_TRACK][1] = "I-IV".to_string();
    app.editor.data[generated][0] = r#"{"generate from chord track": ""}"#.to_string();
    app.editor.data[handwritten][0] = r#"{"generate from chord track": ""}"#.to_string();
    app.editor.data[handwritten][1] = "cde".to_string();
    app.sync_cache_states();

    let affected = app.invalidate_dependent_cells(crate::CHORD_TRACK, 1);

    assert_eq!(affected, vec![(generated, 1)]);
}
