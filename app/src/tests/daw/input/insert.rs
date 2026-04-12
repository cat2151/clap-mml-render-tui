use super::*;

#[test]
fn commit_insert_skips_cache_refresh_when_text_is_unchanged() {
    let tmp = std::env::temp_dir().join("cmrt_test_commit_insert_skips_cache_refresh");
    std::fs::remove_dir_all(&tmp).ok();

    {
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);

        let (mut app, cache_rx) = build_test_app();
        app.data[1][1] = "cdef".to_string();
        {
            let mut cache = app.cache.lock().unwrap();
            cache[1][1].state = CacheState::Ready;
            cache[1][1].generation = 7;
        }

        app.start_insert();
        app.commit_insert();

        let cache = app.cache.lock().unwrap();
        assert_eq!(app.data[1][1], "cdef");
        assert!(matches!(cache[1][1].state, CacheState::Ready));
        assert_eq!(cache[1][1].generation, 7);
        assert!(
            cache_rx.try_recv().is_err(),
            "unchanged insert queued a cache job"
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn handle_insert_ctrl_c_copies_selected_text() {
    let (mut app, _cache_rx) = build_test_app();
    app.mode = DawMode::Insert;
    app.textarea = TextArea::from(["Hello World"]);
    assert_eq!(crate::clipboard::take_text_for_test(), None);
    app.textarea.move_cursor(CursorMove::WordForward);
    app.textarea.start_selection();
    app.textarea.move_cursor(CursorMove::End);

    app.handle_insert(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(app.textarea.yank_text(), "World");
    assert_eq!(app.textarea.lines().join(""), "Hello World");
    assert_eq!(
        crate::clipboard::take_text_for_test(),
        Some("World".to_string())
    );
}

#[test]
fn commit_insert_triggers_cache_refresh_when_text_changes() {
    let tmp = std::env::temp_dir().join("cmrt_test_commit_insert_refreshes_cache");
    std::fs::remove_dir_all(&tmp).ok();

    {
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);

        let (mut app, cache_rx) = build_test_app();
        app.data[1][1] = "cdef".to_string();
        {
            let mut cache = app.cache.lock().unwrap();
            cache[1][1].state = CacheState::Ready;
            cache[1][1].generation = 7;
        }

        app.start_insert();
        app.textarea = TextArea::default();
        for ch in "gfed".chars() {
            app.textarea.insert_char(ch);
        }
        app.commit_insert();

        let cache = app.cache.lock().unwrap();
        assert_eq!(app.data[1][1], "gfed");
        assert!(matches!(cache[1][1].state, CacheState::Rendering));
        assert_eq!(cache[1][1].generation, 8);

        let job = cache_rx
            .try_recv()
            .expect("changed insert did not queue a cache job");
        assert_eq!(job.track, 1);
        assert_eq!(job.measure, 1);
        assert_eq!(job.generation, 8);
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn commit_insert_keeps_semicolon_text_in_same_measure() {
    let tmp = std::env::temp_dir().join("cmrt_test_commit_insert_keeps_semicolon_text");
    std::fs::remove_dir_all(&tmp).ok();

    {
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);

        let (mut app, cache_rx) = build_test_app();
        app.data[0][0] = r#"{"beat": "4/4"}t120"#.to_string();
        app.data[1][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
        app.data[2][1] = "existing".to_string();

        app.start_insert();
        app.textarea = TextArea::default();
        for ch in "cde;gab".chars() {
            app.textarea.insert_char(ch);
        }
        app.commit_insert();

        assert_eq!(app.data[1][1], "cde;gab");
        assert_eq!(app.data[2][1], "existing");

        let job = cache_rx
            .try_recv()
            .expect("semicolon insert did not queue a cache job");
        assert_eq!(job.track, 1);
        assert_eq!(job.measure, 1);
        assert_eq!(
            job.mml.matches(r#"{"Surge XT patch": "piano"}"#).count(),
            2,
            "semicolon-separated phrases should each receive the track timbre: {}",
            job.mml
        );
        assert_eq!(
            job.mml.matches("t120").count(),
            2,
            "semicolon-separated phrases should each receive the track0/header content (t120): {}",
            job.mml
        );
        assert!(
            cache_rx.try_recv().is_err(),
            "unexpected extra cache job queued"
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}
