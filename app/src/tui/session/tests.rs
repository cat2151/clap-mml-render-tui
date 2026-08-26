use super::*;

#[test]
fn daily_daw_is_saved_and_restored_as_the_cold_start_screen() {
    let unique = crate::tui::tests::NEXT_TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_daily_daw_session_restore_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(crate::tui::tests::test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::DailyDaw;
    app.save_history_state();
    drop(app);

    let saved = crate::history::load_session_state();
    assert_eq!(saved.active_screen, crate::history::PrimaryScreen::DailyDaw);
    let cfg = crate::tui::tests::test_config();
    let restored = TuiApp::new(&cfg, cmrt_offline_render::PluginEntries::none());
    assert_eq!(
        restored.active_screen,
        crate::screen_switch::PrimaryScreen::DailyDaw
    );
    assert_eq!(
        crate::tui::runtime::DawEntryRoute::Restored(restored.active_screen).screen(),
        Some(crate::screen_switch::PrimaryScreen::DailyDaw)
    );

    std::fs::remove_dir_all(&tmp).ok();
}
