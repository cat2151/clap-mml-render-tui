use super::*;

use crate::{CellCache, DawGridImportSong, DawGridImportTrack, WorkspaceKind};

fn song(track_volumes_db: Option<Vec<i32>>) -> DawGridImportSong {
    DawGridImportSong {
        bpm: 120.0,
        chord: None,
        track_volumes_db,
        tracks: vec![
            DawGridImportTrack {
                patch: None,
                swing: 50,
                measures: vec!["o5c1".to_string()],
                chord_binding: None,
            },
            DawGridImportTrack {
                patch: None,
                swing: 50,
                measures: vec!["o4c1".to_string()],
                chord_binding: None,
            },
        ],
    }
}

fn imported_daily_app(track_volumes_db: Option<Vec<i32>>) -> DawApp {
    let (mut app, _cache_rx) = crate::input::tests::build_test_app();
    app.workspace_kind = WorkspaceKind::Daily;
    app.replace_with_grid_song(song(track_volumes_db)).unwrap();
    app
}

/// 先頭小節へ、指定した振幅の定数サンプルを持つ Ready な cache を差し込む。
fn put_ready_first_measure(app: &DawApp, track: usize, amplitude: f32) {
    let mut cache = app.cache.lock().unwrap();
    cache[track][AUTO_TRIM_MEASURE] = CellCache {
        state: CacheState::Ready,
        samples: Some(Arc::new(vec![amplitude; 512])),
        rendered_measure_samples: Some(512),
        generation: 0,
        rendered_mml_hash: Some(0),
    };
}

fn set_first_measure_state(app: &DawApp, track: usize, state: CacheState) {
    app.cache.lock().unwrap()[track][AUTO_TRIM_MEASURE].state = state;
}

#[test]
fn preview_measured_volumes_are_used_as_is() {
    let volumes_db = {
        let mut volumes_db = vec![0; FIRST_PLAYABLE_TRACK + 2];
        volumes_db[FIRST_PLAYABLE_TRACK] = -9;
        volumes_db
    };
    let app = imported_daily_app(Some(volumes_db.clone()));

    assert_eq!(app.track_volumes_db, volumes_db);
    // preview で測れているので、先頭小節の render を待つ必要はない。
    assert!(!app.pending_auto_trim);
}

#[test]
fn import_without_preview_waits_for_the_first_measure() {
    let mut app = imported_daily_app(None);
    assert!(app.pending_auto_trim);

    // 片方だけ揃っても決めない。
    put_ready_first_measure(&app, FIRST_PLAYABLE_TRACK, 0.5);
    set_first_measure_state(&app, FIRST_PLAYABLE_TRACK + 1, CacheState::Rendering);
    app.pump_pending_auto_trim();
    assert!(app.pending_auto_trim);
    assert_eq!(app.track_volumes_db, vec![0; FIRST_PLAYABLE_TRACK + 2]);

    put_ready_first_measure(&app, FIRST_PLAYABLE_TRACK + 1, 0.05);
    app.pump_pending_auto_trim();

    assert!(!app.pending_auto_trim);
    // 20dB 差のうち、小さい方を 0dB 基準にして大きい方だけを下げる。
    assert_eq!(app.track_volumes_db[FIRST_PLAYABLE_TRACK], -15);
    assert_eq!(app.track_volumes_db[FIRST_PLAYABLE_TRACK + 1], 0);
}

#[test]
fn evenly_balanced_tracks_keep_the_default_volume() {
    let mut app = imported_daily_app(None);
    put_ready_first_measure(&app, FIRST_PLAYABLE_TRACK, 0.2);
    put_ready_first_measure(&app, FIRST_PLAYABLE_TRACK + 1, 0.2);

    app.pump_pending_auto_trim();

    assert!(!app.pending_auto_trim);
    assert_eq!(app.track_volumes_db, vec![0; FIRST_PLAYABLE_TRACK + 2]);
}

#[test]
fn failed_or_empty_tracks_do_not_block_the_decision() {
    let mut app = imported_daily_app(None);
    put_ready_first_measure(&app, FIRST_PLAYABLE_TRACK, 0.5);
    // render に失敗した track を待ち続けない。
    set_first_measure_state(&app, FIRST_PLAYABLE_TRACK + 1, CacheState::Error);

    app.pump_pending_auto_trim();

    assert!(!app.pending_auto_trim);
    // 測れたのが 1 track だけなら、その track が基準になって 0dB のまま。
    assert_eq!(app.track_volumes_db[FIRST_PLAYABLE_TRACK], 0);
}

#[test]
fn silent_first_measure_leaves_every_track_at_zero_db() {
    let mut app = imported_daily_app(None);
    put_ready_first_measure(&app, FIRST_PLAYABLE_TRACK, 0.0);
    put_ready_first_measure(&app, FIRST_PLAYABLE_TRACK + 1, 0.0);

    app.pump_pending_auto_trim();

    assert!(!app.pending_auto_trim);
    assert_eq!(app.track_volumes_db, vec![0; FIRST_PLAYABLE_TRACK + 2]);
}

#[test]
fn nothing_happens_without_a_pending_request() {
    let mut app = imported_daily_app(Some(vec![-6; FIRST_PLAYABLE_TRACK + 2]));
    put_ready_first_measure(&app, FIRST_PLAYABLE_TRACK, 0.5);
    put_ready_first_measure(&app, FIRST_PLAYABLE_TRACK + 1, 0.05);

    app.pump_pending_auto_trim();

    assert_eq!(app.track_volumes_db, vec![-6; FIRST_PLAYABLE_TRACK + 2]);
}
