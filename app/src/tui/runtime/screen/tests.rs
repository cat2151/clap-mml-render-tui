use super::*;

#[test]
fn daw_entry_screens_select_their_own_workspace() {
    assert_eq!(
        daw_workspace_for_screen(PrimaryScreen::DailyDaw),
        Some(crate::daw::WorkspaceKind::Daily)
    );
    assert_eq!(
        daw_workspace_for_screen(PrimaryScreen::Daw),
        Some(crate::daw::WorkspaceKind::Persistent)
    );
    assert_eq!(daw_workspace_for_screen(PrimaryScreen::Notepad), None);
}

#[test]
fn only_saved_or_screen_switch_daily_entries_select_daily_workspace() {
    for route in [
        DawEntryRoute::Keyboard,
        DawEntryRoute::Notepad,
        DawEntryRoute::Http,
        DawEntryRoute::ScreenSwitch(PrimaryScreen::Daw),
    ] {
        let screen = route.screen().expect("legacy DAW route");
        assert_eq!(
            daw_workspace_for_screen(screen),
            Some(crate::daw::WorkspaceKind::Persistent)
        );
    }
    for route in [
        DawEntryRoute::ScreenSwitch(PrimaryScreen::DailyDaw),
        DawEntryRoute::Restored(PrimaryScreen::DailyDaw),
    ] {
        let screen = route.screen().expect("Daily DAW route");
        assert_eq!(
            daw_workspace_for_screen(screen),
            Some(crate::daw::WorkspaceKind::Daily)
        );
    }
    assert_eq!(
        DawEntryRoute::Restored(PrimaryScreen::Notepad).screen(),
        None
    );
}

#[test]
fn switching_to_daily_daw_records_the_distinct_top_level_screen() {
    let mut app = TuiApp::new_for_test(crate::tui::tests::test_config());

    app.switch_to_primary_screen(PrimaryScreen::DailyDaw, None);

    assert_eq!(app.active_screen, PrimaryScreen::DailyDaw);
    assert_eq!(app.notepad.mode, Mode::Normal);
}
