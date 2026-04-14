use super::*;

#[test]
fn handle_normal_r_uses_saved_patch_filter_query_for_random_selection() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_normal_r_uses_saved_filter");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(tmp.join("Bass")).unwrap();
    std::fs::create_dir_all(tmp.join("Lead")).unwrap();
    std::fs::write(tmp.join("Bass").join("Bass 1.fxp"), b"dummy").unwrap();
    std::fs::write(tmp.join("Bass").join("Bass 2.fxp"), b"dummy").unwrap();
    std::fs::write(tmp.join("Lead").join("Lead 1.fxp"), b"dummy").unwrap();

    {
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);

        let (mut app, _cache_rx) = build_test_app();
        app.cursor_track = 1;
        app.cursor_measure = 0;
        app.cfg = Arc::new(Config {
            patches_dirs: Some(vec![tmp.to_string_lossy().into_owned()]),
            ..(*app.cfg).clone()
        });
        app.data[1][0] =
            r#"{"Surge XT patch":"Lead/Lead 1.fxp","Surge XT patch filter":"bass"}"#.to_string();

        app.handle_normal(crossterm::event::KeyCode::Char('r'));

        let init_json: serde_json::Value = serde_json::from_str(&app.data[1][0]).unwrap();
        let selected_patch = init_json["Surge XT patch"]
            .as_str()
            .expect("selected patch should be stored as string");
        assert!(
            matches!(selected_patch, "Bass/Bass 1.fxp" | "Bass/Bass 2.fxp"),
            "selected patch should respect saved filter query: {selected_patch}"
        );
        assert_eq!(init_json["Surge XT patch filter"], "bass");
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn handle_normal_r_keeps_filter_cycle_unique_for_160_candidates() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_normal_r_unique_cycle_160");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(tmp.join("Pads")).unwrap();

    for i in 1..=160 {
        std::fs::write(tmp.join("Pads").join(format!("Pad {i}.fxp")), b"dummy").unwrap();
    }

    {
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);

        let (mut app, _cache_rx) = build_test_app();
        app.cursor_track = 1;
        app.cursor_measure = 0;
        app.cfg = Arc::new(Config {
            patches_dirs: Some(vec![tmp.to_string_lossy().into_owned()]),
            ..(*app.cfg).clone()
        });
        app.data[1][0] =
            r#"{"Surge XT patch":"Pads/Pad 1.fxp","Surge XT patch filter":"pad"}"#.to_string();

        let mut seen = HashSet::new();
        for _ in 0..160 {
            app.handle_normal(crossterm::event::KeyCode::Char('r'));
            let init_json: serde_json::Value = serde_json::from_str(&app.data[1][0]).unwrap();
            let selected_patch = init_json["Surge XT patch"]
                .as_str()
                .expect("selected patch should be stored as string")
                .to_string();
            assert!(
                seen.insert(selected_patch),
                "selected patch repeated within the same filter cycle"
            );
        }

        assert_eq!(seen.len(), 160);
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn handle_normal_r_keeps_independent_history_per_filter_query() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_normal_r_independent_filter_history");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(tmp.join("Pads")).unwrap();
    std::fs::create_dir_all(tmp.join("Bass")).unwrap();
    std::fs::write(tmp.join("Pads").join("Pad 1.fxp"), b"dummy").unwrap();
    std::fs::write(tmp.join("Pads").join("Pad 2.fxp"), b"dummy").unwrap();
    std::fs::write(tmp.join("Pads").join("Pad 3.fxp"), b"dummy").unwrap();
    std::fs::write(tmp.join("Bass").join("Bass 1.fxp"), b"dummy").unwrap();
    std::fs::write(tmp.join("Bass").join("Bass 2.fxp"), b"dummy").unwrap();

    {
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);

        let (mut app, _cache_rx) = build_test_app();
        app.cursor_track = 1;
        app.cursor_measure = 0;
        app.cfg = Arc::new(Config {
            patches_dirs: Some(vec![tmp.to_string_lossy().into_owned()]),
            ..(*app.cfg).clone()
        });

        app.data[1][0] =
            r#"{"Surge XT patch":"Pads/Pad 1.fxp","Surge XT patch filter":"pad"}"#.to_string();
        app.handle_normal(crossterm::event::KeyCode::Char('r'));
        let pad_first: serde_json::Value = serde_json::from_str(&app.data[1][0]).unwrap();
        let pad_first = pad_first["Surge XT patch"].as_str().unwrap().to_string();

        app.data[1][0] =
            r#"{"Surge XT patch":"Bass/Bass 1.fxp","Surge XT patch filter":"bass"}"#.to_string();
        app.handle_normal(crossterm::event::KeyCode::Char('r'));
        let bass_first: serde_json::Value = serde_json::from_str(&app.data[1][0]).unwrap();
        let bass_first = bass_first["Surge XT patch"].as_str().unwrap().to_string();

        app.data[1][0] =
            format!(r#"{{"Surge XT patch":"{pad_first}","Surge XT patch filter":"pad"}}"#);
        app.handle_normal(crossterm::event::KeyCode::Char('r'));
        let pad_second: serde_json::Value = serde_json::from_str(&app.data[1][0]).unwrap();
        let pad_second = pad_second["Surge XT patch"].as_str().unwrap().to_string();

        app.data[1][0] =
            format!(r#"{{"Surge XT patch":"{bass_first}","Surge XT patch filter":"bass"}}"#);
        app.handle_normal(crossterm::event::KeyCode::Char('r'));
        let bass_second: serde_json::Value = serde_json::from_str(&app.data[1][0]).unwrap();
        let bass_second = bass_second["Surge XT patch"].as_str().unwrap().to_string();

        assert_ne!(pad_first, pad_second);
        assert_ne!(bass_first, bass_second);
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn handle_normal_r_preserves_trailing_init_mml_when_updating_patch_json() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_normal_r_preserves_trailing_init_mml");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(tmp.join("Pad")).unwrap();
    std::fs::write(tmp.join("Pad").join("Pad 1.fxp"), b"dummy").unwrap();

    {
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);

        let (mut app, _cache_rx) = build_test_app();
        app.cursor_track = 1;
        app.cursor_measure = 0;
        app.cfg = Arc::new(Config {
            patches_dirs: Some(vec![tmp.to_string_lossy().into_owned()]),
            ..(*app.cfg).clone()
        });
        app.data[1][0] =
            r#"{"Surge XT patch":"Old/Lead 1.fxp","Surge XT patch filter":"pad","custom":"keep"}l1"#.to_string();
        app.data[1][1] = "cdef".to_string();

        app.handle_normal(crossterm::event::KeyCode::Char('r'));

        assert_eq!(
            app.data[1][0],
            r#"{"Surge XT patch":"Pad/Pad 1.fxp","Surge XT patch filter":"pad","custom":"keep"}l1"#
        );
        let play_measure_track_mmls = app.play_measure_track_mmls.lock().unwrap().clone();
        assert!(
            play_measure_track_mmls[0][1].contains("l1cdef"),
            "updated init MML should keep the trailing phrase in playback state: {:?}",
            play_measure_track_mmls
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn handle_normal_r_preserves_init_json_formatting_and_whitespace() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_normal_r_preserves_init_json_formatting");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(tmp.join("Pad")).unwrap();
    std::fs::write(tmp.join("Pad").join("Pad 1.fxp"), b"dummy").unwrap();

    {
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);

        let (mut app, _cache_rx) = build_test_app();
        app.cursor_track = 1;
        app.cursor_measure = 0;
        app.cfg = Arc::new(Config {
            patches_dirs: Some(vec![tmp.to_string_lossy().into_owned()]),
            ..(*app.cfg).clone()
        });
        app.data[1][0] =
            r#"{ "custom" : "keep" , "Surge XT patch" : "Old/Lead 1.fxp" , "Surge XT patch filter" : "pad" }  l1 "#.to_string();
        app.data[1][1] = "cdef".to_string();

        app.handle_normal(crossterm::event::KeyCode::Char('r'));

        assert_eq!(
            app.data[1][0],
            r#"{ "custom" : "keep" , "Surge XT patch" : "Pad/Pad 1.fxp" , "Surge XT patch filter" : "pad" }  l1 "#
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn replace_patch_name_in_mml_preserves_escaped_json_strings() {
    let current =
        r#"{ "memo" : "quote \" and slash \\ keep" , "Surge XT patch" : "Old/Lead 1.fxp" }  l1"#;

    let replaced = DawApp::replace_patch_name_in_mml(current, r#"Pad/"A"\B.fxp"#, None);

    assert_eq!(
        replaced,
        r#"{ "memo" : "quote \" and slash \\ keep" , "Surge XT patch" : "Pad/\"A\"\\B.fxp" }  l1"#
    );
}
