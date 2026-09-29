use super::*;

fn prefetched(app: &NotepadScreen<'_>) -> Vec<String> {
    app.audio.order.lock().unwrap().iter().cloned().collect()
}

fn tone_mml(index: usize) -> String {
    format!(r#"{{"Surge XT patch": "Tone {index:02}"}} l8cdef"#)
}

#[test]
fn j_prefetches_the_moving_direction_first_then_the_remaining_navigation_targets() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 24, "Tone 04");
    app.audio.order.lock().unwrap().clear();
    app.audio.cache.lock().unwrap().clear();

    press(&mut app, KeyCode::Char('j'));

    let order = prefetched(&app);
    assert_eq!(order[..2], [tone_mml(6), tone_mml(7)], "{order:#?}");
    assert!(
        order.contains(&tone_mml(15)),
        "page down target: {order:#?}"
    );
    assert!(
        !order.contains(&tone_mml(5)),
        "the previewed patch is not a prefetch target"
    );
}

#[test]
fn k_prefetches_upward_first() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 24, "Tone 12");
    app.audio.order.lock().unwrap().clear();
    app.audio.cache.lock().unwrap().clear();

    press(&mut app, KeyCode::Char('k'));

    let order = prefetched(&app);
    assert_eq!(order[..2], [tone_mml(10), tone_mml(9)], "{order:#?}");
    assert!(order.contains(&tone_mml(1)), "page up target: {order:#?}");
}

#[test]
fn a_heavy_neighbor_is_not_prefetched_but_light_ones_still_are() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones_with_heavy(&mut app, 24, "Tone 04", "Tone 06");
    app.audio.order.lock().unwrap().clear();
    app.audio.cache.lock().unwrap().clear();

    press(&mut app, KeyCode::Char('j'));

    let order = prefetched(&app);
    assert!(!order.contains(&tone_mml(6)), "heavy patch: {order:#?}");
    assert_eq!(order[0], tone_mml(7), "{order:#?}");
    assert!(
        order.contains(&tone_mml(15)),
        "page down target: {order:#?}"
    );
}
