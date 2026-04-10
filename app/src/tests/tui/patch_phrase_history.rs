use super::*;

#[test]
fn record_patch_phrase_history_uses_phrase_without_embedded_json() {
    let mut app = TuiApp::new_for_test(test_config());

    app.record_patch_phrase_history(r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#);
    app.record_patch_phrase_history(r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8efga"#);
    app.record_patch_phrase_history(r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#);

    let stored = app
        .patch_phrase_store
        .patches
        .get("Pads/Pad 1.fxp")
        .expect("patch history should be stored");
    assert_eq!(
        stored.history,
        vec!["l8cdef".to_string(), "l8efga".to_string()]
    );
    assert!(stored.favorites.is_empty());
}

#[test]
fn record_patch_phrase_history_truncates_to_recent_100_items() {
    let mut app = TuiApp::new_for_test(test_config());

    for i in 0..105 {
        app.record_patch_phrase_history(&format!(
            r#"{{"Surge XT patch":"Pads/Pad 1.fxp"}} l8c{}"#,
            i
        ));
    }

    let stored = app
        .patch_phrase_store
        .patches
        .get("Pads/Pad 1.fxp")
        .expect("patch history should be stored");
    assert!(app.patch_phrase_store_dirty);
    assert_eq!(stored.history.len(), 100);
    assert_eq!(stored.history.first().map(String::as_str), Some("l8c104"));
    assert_eq!(stored.history.last().map(String::as_str), Some("l8c5"));
}

#[test]
fn record_patch_phrase_history_resolves_factory_prefixed_patch_name() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "patches_factory/Pads/Pad 1.fxp",
    ]))));

    app.record_patch_phrase_history(r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#);

    let stored = app
        .patch_phrase_store
        .patches
        .get("patches_factory/Pads/Pad 1.fxp")
        .expect("patch history should be stored with prefixed patch name");
    assert_eq!(stored.history, vec!["l8cdef".to_string()]);
    assert!(!app
        .patch_phrase_store
        .patches
        .contains_key("Pads/Pad 1.fxp"));
}

#[test]
fn start_patch_phrase_migrates_existing_history_and_favorites_to_prefixed_patch_name() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "patches_factory/Pads/Pad 1.fxp",
    ]))));
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["hist".to_string()],
            favorites: vec!["fav".to_string()],
        },
    );

    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    assert_eq!(
        app.patch_phrase_name.as_deref(),
        Some("patches_factory/Pads/Pad 1.fxp")
    );
    let stored = app
        .patch_phrase_store
        .patches
        .get("patches_factory/Pads/Pad 1.fxp")
        .expect("migrated patch state should exist");
    assert_eq!(stored.history, vec!["hist".to_string()]);
    assert_eq!(stored.favorites, vec!["fav".to_string()]);
    assert!(!app
        .patch_phrase_store
        .patches
        .contains_key("Pads/Pad 1.fxp"));
}

#[test]
fn patch_phrase_store_flushes_only_when_requested() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_patch_phrase_flush_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.record_patch_phrase_history(r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#);

    let patch_history_path = crate::test_utils::patch_phrase_store_path_for_test()
        .expect("config local dir should resolve in isolated test");
    assert!(
        !patch_history_path.exists(),
        "patch history should not be written until flush is requested"
    );
    assert!(app.patch_phrase_store_dirty);

    app.flush_patch_phrase_store_if_dirty();

    assert!(patch_history_path.exists());
    assert!(!app.patch_phrase_store_dirty);
    let loaded = crate::history::load_patch_phrase_store();
    let stored = loaded
        .patches
        .get("Pads/Pad 1.fxp")
        .expect("flushed patch history should be persisted");
    assert_eq!(stored.history, vec!["l8cdef".to_string()]);

    std::fs::remove_dir_all(&tmp).ok();
}
