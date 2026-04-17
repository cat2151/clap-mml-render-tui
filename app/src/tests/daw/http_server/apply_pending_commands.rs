use super::*;

#[test]
fn apply_pending_http_commands_updates_mml_and_expands_grid() {
    let _test_guard = lock_http_server_test_state();
    let tmp = std::env::temp_dir().join("cmrt_test_http_server_updates_mml");
    std::fs::remove_dir_all(&tmp).ok();
    let _guard = crate::test_utils::set_local_dir_envs(&tmp);

    let cfg = default_config();
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(
        &state,
        DawHttpCommandKind::Mml {
            track: 3,
            measure: 4,
            mml: "l8cde".to_string(),
        },
    );

    let mut app = build_test_app(cfg);
    app.apply_pending_http_commands();

    assert_eq!(app.tracks, 4);
    assert_eq!(app.measures, 4);
    assert_eq!(app.data[3][4], "l8cde");
    assert_eq!(state.lock().unwrap().grid_snapshot[3][4], "l8cde");
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));

    deactivate_daw_http_server();
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn apply_pending_http_commands_updates_mixer_gain() {
    let _test_guard = lock_http_server_test_state();
    let tmp = std::env::temp_dir().join("cmrt_test_http_server_updates_mixer");
    std::fs::remove_dir_all(&tmp).ok();
    let _guard = crate::test_utils::set_local_dir_envs(&tmp);

    let cfg = default_config();
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(&state, DawHttpCommandKind::Mixer { track: 4, db: -6.0 });

    let mut app = build_test_app(cfg);
    app.apply_pending_http_commands();

    assert_eq!(app.track_volume_db(4), -6);
    assert_eq!(
        app.play_track_gains.lock().unwrap()[4],
        10.0f32.powf(-6.0 / 20.0)
    );
    assert_eq!(state.lock().unwrap().grid_snapshot.len(), 5);
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));

    deactivate_daw_http_server();
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn apply_pending_http_commands_updates_patch_init_cell() {
    let _test_guard = lock_http_server_test_state();
    let tmp = std::env::temp_dir().join("cmrt_test_http_server_updates_patch");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(tmp.join("Pads")).unwrap();
    std::fs::write(tmp.join("Pads").join("Factory Pad.fxp"), b"dummy").unwrap();
    let _guard = crate::test_utils::set_local_dir_envs(&tmp);

    let mut cfg = default_config();
    cfg.patches_dirs = Some(vec![tmp.to_string_lossy().into_owned()]);
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(
        &state,
        DawHttpCommandKind::Patch {
            track: 1,
            patch: "Pads/Factory Pad.fxp".to_string(),
        },
    );

    let mut app = build_test_app(cfg);
    app.apply_pending_http_commands();

    assert_eq!(
        app.data[1][0],
        DawApp::build_patch_json("Pads/Factory Pad.fxp")
    );
    assert_eq!(
        state.lock().unwrap().grid_snapshot[1][0],
        DawApp::build_patch_json("Pads/Factory Pad.fxp")
    );
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));

    deactivate_daw_http_server();
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn apply_pending_http_commands_updates_random_patch_init_cell() {
    let _test_guard = lock_http_server_test_state();
    let tmp = std::env::temp_dir().join("cmrt_test_http_server_updates_random_patch");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(tmp.join("Pad")).unwrap();
    std::fs::write(tmp.join("Pad").join("Pad 1.fxp"), b"dummy").unwrap();
    let _guard = crate::test_utils::set_local_dir_envs(&tmp);

    let mut cfg = default_config();
    cfg.patches_dirs = Some(vec![tmp.to_string_lossy().into_owned()]);
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(&state, DawHttpCommandKind::RandomPatch { track: 1 });

    let mut app = build_test_app(cfg);
    app.data[1][0] =
        r#"{"Surge XT patch":"Old/Lead 1.fxp","Surge XT patch filter":"pad","custom":"keep"}l1"#
            .to_string();
    app.data[1][1] = "l8cde".to_string();
    app.apply_pending_http_commands();

    assert_eq!(
        app.data[1][0],
        r#"{"Surge XT patch":"Pad/Pad 1.fxp","Surge XT patch filter":"pad","custom":"keep"}l1"#
    );
    assert_eq!(
        state.lock().unwrap().grid_snapshot[1][0],
        r#"{"Surge XT patch":"Pad/Pad 1.fxp","Surge XT patch filter":"pad","custom":"keep"}l1"#
    );
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));
    assert!(app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == "http: patch/random track=1"));

    deactivate_daw_http_server();
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn apply_random_patch_to_track_rejects_out_of_range_track() {
    let cfg = default_config();
    let mut app = build_test_app(cfg);

    let result = app.apply_random_patch_to_track(usize::MAX);

    assert_eq!(
        result,
        Err("track は 1..=2 の範囲で指定してください".to_string())
    );
}

#[test]
fn apply_pending_http_commands_starts_play() {
    let _test_guard = lock_http_server_test_state();
    let cfg = default_config();
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(&state, DawHttpCommandKind::PlayStart);

    let mut app = build_test_app(cfg);
    app.data[1][1] = "l8c".to_string();
    app.apply_pending_http_commands();

    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Playing
    ));
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));
    assert!(app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == "play: start"));
    assert!(app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == "http: play start"));

    deactivate_daw_http_server();
}

#[test]
fn apply_pending_http_commands_start_while_playing_is_noop() {
    let _test_guard = lock_http_server_test_state();
    let cfg = default_config();
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(&state, DawHttpCommandKind::PlayStart);

    let mut app = build_test_app(cfg);
    *app.play_state.lock().unwrap() = DawPlayState::Playing;
    app.apply_pending_http_commands();

    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Playing
    ));
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("http: play start (already playing)")
    );

    deactivate_daw_http_server();
}

#[test]
fn apply_pending_http_commands_start_without_playable_data_returns_error() {
    let _test_guard = lock_http_server_test_state();
    let cfg = default_config();
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(&state, DawHttpCommandKind::PlayStart);

    let mut app = build_test_app(cfg);
    app.apply_pending_http_commands();

    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert_eq!(
        response_rx.try_recv().unwrap(),
        Err("再生可能なデータがありません".to_string())
    );
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("http: play start (no playable data)")
    );

    deactivate_daw_http_server();
}

#[test]
fn apply_pending_http_commands_stops_play() {
    let _test_guard = lock_http_server_test_state();
    let cfg = default_config();
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(&state, DawHttpCommandKind::PlayStop);

    let mut app = build_test_app(cfg);
    *app.play_state.lock().unwrap() = DawPlayState::Playing;
    app.apply_pending_http_commands();

    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));
    assert!(app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == "play: stop"));
    assert!(app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == "http: play stop"));

    deactivate_daw_http_server();
}

#[test]
fn apply_pending_http_commands_updates_ab_repeat_range() {
    let _test_guard = lock_http_server_test_state();
    let cfg = default_config();
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(
        &state,
        DawHttpCommandKind::AbRepeat {
            start_measure: 1,
            end_measure: 2,
        },
    );

    let mut app = build_test_app(cfg);
    app.apply_pending_http_commands();

    assert_eq!(
        app.ab_repeat_state(),
        AbRepeatState::FixEnd {
            start_measure_index: 0,
            end_measure_index: 1,
        }
    );
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));

    deactivate_daw_http_server();
}

#[test]
fn apply_http_mml_rejects_measure_index_overflow() {
    let cfg = default_config();
    let mut app = build_test_app(cfg);

    let result = app.apply_http_mml(1, usize::MAX, "c");

    assert_eq!(result, Err("measure index が大きすぎます".to_string()));
}

#[test]
fn apply_http_ab_repeat_rejects_init_column_and_out_of_range_measure() {
    let cfg = default_config();
    let mut app = build_test_app(cfg);

    assert_eq!(
        app.apply_http_ab_repeat(0, 1),
        Err("measA と measB は 1 以上を指定してください".to_string())
    );
    assert_eq!(
        app.apply_http_ab_repeat(1, 3),
        Err("measA と measB は 1..=2 の範囲で指定してください".to_string())
    );
}
