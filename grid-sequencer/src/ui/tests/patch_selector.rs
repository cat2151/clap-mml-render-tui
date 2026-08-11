use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn render_note_grid(screen: &GridSequencerScreen) -> String {
    let mut terminal = Terminal::new(TestBackend::new(90, 24)).unwrap();
    let connection = screen.connection_status();
    terminal
        .draw(|frame| {
            let area = frame.area();
            crate::ui::grid::draw(screen, &connection, frame, area);
        })
        .unwrap();
    buffer_to_string(&terminal)
}

#[test]
fn the_patch_selector_overlay_shows_categories_and_patches() {
    let patches = vec![
        ("Keys/Alpha.fxp".to_string(), "keys/alpha.fxp".to_string()),
        ("Keys/Beta.fxp".to_string(), "keys/beta.fxp".to_string()),
    ];
    let ctx = crate::tests::ctx_with(
        crate::GridPatchLoad::Ready(&patches),
        crate::tests::empty_catalog(),
        &crate::NoVoicingLookup,
    );
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());
    screen.open_patch_selector(0, &ctx);

    let rendered = render(&screen);

    assert!(rendered.contains("instance 1 patch select"), "{rendered}");
    assert!(rendered.contains("Categories (1)"), "{rendered}");
    assert!(rendered.contains("Keys/Alpha.fxp"), "{rendered}");
    assert!(rendered.contains("Keys/Beta.fxp"), "{rendered}");
    assert!(rendered.contains("wheel/↑↓:preview"), "{rendered}");
    assert!(rendered.contains("r:random"), "{rendered}");
    assert!(rendered.contains("click/Enter:apply"), "{rendered}");
}

#[test]
fn the_grid_patch_name_follows_keyboard_and_random_previews_without_committing() {
    let patches = vec![
        ("Keys/Alpha.fxp".to_string(), "keys/alpha.fxp".to_string()),
        ("Keys/Beta.fxp".to_string(), "keys/beta.fxp".to_string()),
    ];
    let ctx = crate::tests::ctx_with(
        crate::GridPatchLoad::Ready(&patches),
        crate::tests::empty_catalog(),
        &crate::NoVoicingLookup,
    );
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());
    screen.open_patch_selector(0, &ctx);

    screen.handle_patch_selector_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &ctx);
    assert!(render_note_grid(&screen).contains("Keys/Beta.fxp"));
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp"),
        "preview 中はまだ commit しない"
    );

    screen.handle_patch_selector_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE), &ctx);
    assert!(render_note_grid(&screen).contains("Keys/Alpha.fxp"));
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp"),
        "r preview でも state は未確定のまま"
    );
}

#[test]
fn a_manual_patch_load_greys_only_the_target_row() {
    let screen = GridSequencerScreen::with_track_count(None, 2);
    let connection = GridConnectionStatus {
        phase: GridConnectionPhase::Ready,
        row_patch: Some(crate::GridRowPatchStatus {
            row: 1,
            phase: crate::GridRowPatchPhase::Loading,
        }),
        ..GridConnectionStatus::default()
    };

    let terminal = terminal_with_connection(&screen, &connection);
    let rendered = render_with_connection(&screen, &connection);

    assert_eq!(row_label_fg(&terminal, &screen, 0), MONOKAI_FG);
    assert_eq!(row_label_fg(&terminal, &screen, 1), MONOKAI_GRAY);
    assert!(rendered.contains("instance 2 patch loading"), "{rendered}");
    assert!(!has_progress_overlay(&rendered), "{rendered}");
}
