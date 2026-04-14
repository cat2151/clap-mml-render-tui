use super::*;

#[test]
fn handle_normal_g_sets_random_patch_and_generated_phrase_then_previews() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_normal_g_generate");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();
    let patch_path = tmp.join("Pad 1.fxp");
    std::fs::write(&patch_path, b"dummy").unwrap();

    {
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);

        let (mut app, _cache_rx) = build_test_app();
        app.cfg = Arc::new(Config {
            patches_dirs: Some(vec![tmp.to_string_lossy().into_owned()]),
            ..(*app.cfg).clone()
        });
        app.cursor_track = 1;
        app.cursor_measure = 1;
        app.data[1][0] = r#"{"Surge XT patch":"Old/Pad.fxp"}"#.to_string();
        app.data[1][1] = "old phrase".to_string();

        let result = app.handle_normal(crossterm::event::KeyCode::Char('g'));

        assert!(matches!(result, super::super::DawNormalAction::Continue));
        assert_eq!(app.data[1][0], r#"{"Surge XT patch":"Pad 1.fxp"}"#);
        assert!(
            app.data[1][1] == "c1" || app.data[1][1] == "cfg1",
            "generated phrase: {}",
            app.data[1][1]
        );
        assert_eq!(
            app.patch_phrase_store
                .patches
                .get("Old/Pad.fxp")
                .map(|state| state.history.clone()),
            Some(vec!["old phrase".to_string()])
        );
        assert!(matches!(
            *app.play_state.lock().unwrap(),
            DawPlayState::Preview
        ));
        assert_eq!(
            app.log_lines.lock().unwrap().back().map(String::as_str),
            Some("preview: meas1")
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn handle_normal_g_rejects_init_column() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 0;

    let result = app.handle_normal(crossterm::event::KeyCode::Char('g'));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("generate は init 以外の小節でのみ使用できます")
    );
}

#[test]
fn skips_history_when_generate_is_noop() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch":"Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "c1".to_string();

    app.apply_generate_to_current_measure_with("Pad 1.fxp".to_string(), "c1", 0);

    assert_eq!(app.data[1][0], r#"{"Surge XT patch":"Pad 1.fxp"}"#);
    assert_eq!(app.data[1][1], "c1");
    assert!(app.patch_phrase_store.patches.is_empty());
    assert!(!app.patch_phrase_store_dirty);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
}
