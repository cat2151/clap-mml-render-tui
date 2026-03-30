use super::*;

#[test]
fn draw_shows_history_overlay_title_and_items() {
    let mut app = build_test_app();
    app.mode = DawMode::History;
    app.history_overlay_patch_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );

    let normalized_lines: Vec<String> = render_lines(&app, 100, 30)
        .into_iter()
        .map(|line| line.to_lowercase())
        .collect();

    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("patch history - pads/pad 1.fxp")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines.iter().any(|line| line.contains("l8cdef")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("favorites")),
        "lines: {:?}",
        normalized_lines
    );
}

#[test]
fn draw_shows_patch_select_overlay_title_and_items() {
    let mut app = build_test_app();
    app.mode = DawMode::PatchSelect;
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass/Bass 1.fxp".to_string(), "bass/bass 1.fxp".to_string()),
    ];
    app.patch_filtered = app.patch_all.iter().map(|(orig, _)| orig.clone()).collect();
    app.patch_favorite_items = vec!["Pads/Pad 1.fxp".to_string()];

    let normalized_lines: Vec<String> = render_lines(&app, 100, 30)
        .into_iter()
        .map(|line| line.to_lowercase())
        .collect();

    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("patch select")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("pads/pad 1.fxp")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("favorite音")),
        "lines: {:?}",
        normalized_lines
    );
}
