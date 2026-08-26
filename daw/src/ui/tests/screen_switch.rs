use super::*;

#[test]
fn daw_draws_the_shared_screen_switch_overlay() {
    let mut app = build_test_app();
    app.overlays.screen_switch.open();

    let screen = render_lines(&app, 80, 20).join("\n");

    assert!(screen.contains("Screen Switch"));
    assert!(screen.contains("[N] Notepad"));
    assert!(screen.contains("[A] Daily DAW"));
    assert!(screen.contains("[D] DAW"));
    assert!(screen.contains("[K] Keyboard"));
    assert!(screen.contains("[L] Loop Browser"));
    assert!(screen.contains("[G] Grid Sequencer"));
}

#[test]
fn daw_screen_switch_highlights_the_active_workspace() {
    for (workspace_kind, expected) in [
        (crate::WorkspaceKind::Persistent, "[D] DAW"),
        (crate::WorkspaceKind::Daily, "[A] Daily DAW"),
    ] {
        let mut app = build_test_app();
        app.workspace_kind = workspace_kind;
        app.overlays.screen_switch.open();

        let highlighted = render_buffer(&app, 80, 20)
            .content
            .iter()
            .filter(|cell| {
                cell.fg == cmrt_tui_core::theme::MONOKAI_YELLOW
                    && cell.modifier.contains(ratatui::style::Modifier::BOLD)
            })
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert_eq!(highlighted, expected);
    }
}
