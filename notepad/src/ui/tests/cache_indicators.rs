use super::*;

#[test]
fn normal_screen_shows_active_parallel_render_count_in_purple() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.test_set_active_parallel_render_count(2);

    let buffer = render_buffer(&mut app, 120, 9);
    let (x, y) = find_text(&buffer, "render:");

    assert_eq!(buffer.cell((x, y)).unwrap().fg, MONOKAI_PURPLE);
}

#[test]
fn normal_screen_marks_cached_lines_with_music_note() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["abc".to_string(), "def".to_string()];
    app.audio
        .cache
        .lock()
        .unwrap()
        .insert("abc".to_string(), vec![0.1, 0.2]);

    let screen = render_lines(&mut app, 80, 8).join("\n");

    assert!(screen.contains("▶ ♪ abc"));
    assert!(screen.contains("  def"));
}

#[test]
fn normal_screen_marks_disk_only_cached_lines_with_music_note() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["abc".to_string(), "def".to_string()];
    // "abc" だけディスクキャッシュのハッシュ集合に入っている状態を模擬する。
    // audio_cache（オンメモリ）には積まない ＝ LRUから追い出された後の状態に相当する。
    app.audio
        .known_disk_hashes
        .lock()
        .unwrap()
        .insert(cmrt_history::daw_cache_mml_hash("abc"));

    let screen = render_lines(&mut app, 80, 8).join("\n");

    assert!(screen.contains("▶ ♪ abc"));
    assert!(screen.contains("  def"));
}

#[test]
fn normal_mode_startup_prime_caches_current_line_and_navigation_targets() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![
        "m0".to_string(),
        "m1".to_string(),
        "m2".to_string(),
        "m3".to_string(),
    ];
    app.editor.cursor = 1;
    app.editor.list_state.select(Some(1));
    app.editor.page_size = 2;

    app.prime_normal_mode_startup_cache();

    let cache = app.audio.cache.lock().unwrap();
    assert!(cache.contains_key("m1"));
    assert!(cache.contains_key("m2"));
    assert!(cache.contains_key("m0"));
    assert!(cache.contains_key("m3"));
}

#[test]
fn normal_screen_marks_rendering_lines_with_dots_before_music_note() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["abc".to_string()];
    app.test_set_render_job_status(
        "abc",
        Some(crate::render_queue::TuiRenderJobStatus::Pending),
    );

    let screen = render_lines(&mut app, 80, 8).join("\n");

    assert!(screen.contains("▶ . abc"));
}
