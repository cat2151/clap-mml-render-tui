use super::*;

#[test]
fn loop_status_label_single_measure_shows_single_measure_loop() {
    let mut mmls = vec![String::new(); MEASURES];
    mmls[0] = "c".to_string();

    assert_eq!(
        loop_status_label(&mmls),
        Some("loop: meas1のみ (1小節)".to_string())
    );
}

#[test]
fn loop_status_label_uses_last_non_empty_measure() {
    let mut mmls = vec![String::new(); MEASURES];
    mmls[0] = "c".to_string();
    mmls[2] = "g".to_string();

    assert_eq!(
        loop_status_label(&mmls),
        Some("loop: meas1〜meas3 (3小節)".to_string())
    );
}

#[test]
fn loop_status_label_all_empty_returns_none() {
    assert_eq!(loop_status_label(&vec![String::new(); MEASURES]), None);
}

#[test]
fn loop_measure_summary_label_lists_loop_and_empty_ranges() {
    let mut mmls = vec![String::new(); MEASURES];
    mmls[0] = "c".to_string();

    assert_eq!(
        loop_measure_summary_label(&mmls),
        Some("loop meas : meas 1, empty meas : meas 2～8".to_string())
    );
}

#[test]
fn cache_indicator_uses_single_dot_for_uncached_cells() {
    assert_eq!(cache_indicator(&CacheState::Pending, 0), ".    ");
    assert_eq!(cache_indicator(&CacheState::Pending, 2), ".    ");
}

#[test]
fn cache_indicator_animates_only_while_rendering() {
    assert_eq!(cache_indicator(&CacheState::Rendering, 0), ".    ");
    assert_eq!(cache_indicator(&CacheState::Rendering, 1), "..   ");
    assert_eq!(cache_indicator(&CacheState::Rendering, 2), "...  ");
}

#[test]
fn cache_text_color_keeps_uncached_mml_visible() {
    assert_eq!(cache_text_color(&CacheState::Pending), MONOKAI_FG);
    assert_eq!(cache_text_color(&CacheState::Rendering), MONOKAI_FG);
}

#[test]
fn cache_indicator_color_keeps_pending_animation_visible() {
    assert_eq!(cache_indicator_color(&CacheState::Empty), MONOKAI_GRAY);
    assert_eq!(cache_indicator_color(&CacheState::Pending), MONOKAI_FG);
    assert_eq!(cache_indicator_color(&CacheState::Rendering), MONOKAI_FG);
    assert_eq!(cache_indicator_color(&CacheState::Ready), MONOKAI_GRAY);
    assert_eq!(cache_indicator_color(&CacheState::Error), Color::Red);
}
