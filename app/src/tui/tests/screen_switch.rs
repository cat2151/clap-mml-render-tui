use super::*;
use crate::screen_switch::PrimaryScreen;

fn ctrl_g() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL)
}

#[test]
fn ctrl_g_opens_menu_on_each_tui_primary_screen() {
    let mut app = TuiApp::new_for_test(test_config());
    assert!(app.try_open_screen_switch_menu(ctrl_g()));
    assert!(app.screen_switch_menu.is_open());

    app.screen_switch_menu
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.start_keyboard(None);
    assert!(app.try_open_screen_switch_menu(ctrl_g()));

    app.screen_switch_menu
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.finish_keyboard();
    app.begin_loop_browser_startup();
    app.loop_browser.state.starting = false;
    assert!(app.try_open_screen_switch_menu(ctrl_g()));

    app.screen_switch_menu
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.stop_loop_browser();
    app.enter_grid_sequencer();
    assert!(app.try_open_screen_switch_menu(ctrl_g()));
}

/// grid sequencer は入った時点から常時再生する。help 表示中だけは Ctrl+G を塞ぐ。
#[test]
fn entering_grid_sequencer_starts_playing_and_help_blocks_the_menu() {
    let mut app = TuiApp::new_for_test(test_config());

    app.switch_to_primary_screen(PrimaryScreen::GridSequencer, None);

    assert_eq!(app.active_screen, PrimaryScreen::GridSequencer);
    assert!(app.grid_sequencer.state.is_running());

    app.grid_sequencer.help_open = true;
    assert!(!app.try_open_screen_switch_menu(ctrl_g()));
}

#[test]
fn only_the_grid_sequencer_requests_mouse_capture() {
    let mut app = TuiApp::new_for_test(test_config());
    assert!(!app.uses_mouse_capture());

    app.switch_to_primary_screen(PrimaryScreen::GridSequencer, None);
    assert!(app.uses_mouse_capture());

    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);
    assert!(!app.uses_mouse_capture());
}

/// 画面を離れるときに再生を止めないと、音が鳴りっぱなしになる。
#[test]
fn leaving_grid_sequencer_stops_the_progression() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::GridSequencer, None);

    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);

    assert_eq!(app.active_screen, PrimaryScreen::Notepad);
    assert!(!app.grid_sequencer.state.is_running());
    assert_eq!(app.grid_sequencer.state.step_index(), 0);
}

/// 一度作った grid は、他の画面を経由して戻っても残っている。
#[test]
fn returning_to_grid_sequencer_keeps_the_previous_grid() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::GridSequencer, None);
    let grid = app.grid_sequencer.state.instances().to_vec();

    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);
    app.switch_to_primary_screen(PrimaryScreen::GridSequencer, None);

    assert_eq!(app.grid_sequencer.state.instances(), grid.as_slice());
    assert!(app.grid_sequencer.state.is_running());
}

/// 前回 grid sequencer で終了したセッションを復元した場合、`switch_to_primary_screen`
/// を通らないので、起動時フックがないと無音のまま止まってしまう。
#[test]
fn a_restored_grid_sequencer_session_starts_playing_on_launch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = PrimaryScreen::GridSequencer;
    assert!(!app.grid_sequencer.state.is_running());

    app.enter_restored_grid_sequencer();

    assert!(app.grid_sequencer.state.is_running());
}

#[test]
fn q_quits_from_the_grid_sequencer_screen() {
    use crate::tui::grid_sequencer::GridSequencerAction;

    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::GridSequencer, None);

    let action =
        app.handle_grid_sequencer_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));

    assert!(matches!(action, GridSequencerAction::Quit));
}

#[test]
fn menu_is_unavailable_in_non_normal_states() {
    let mut app = TuiApp::new_for_test(test_config());
    app.notepad.mode = Mode::Insert;
    assert!(!app.try_open_screen_switch_menu(ctrl_g()));

    app.active_screen = crate::screen_switch::PrimaryScreen::LoopBrowser;
    app.active_screen = PrimaryScreen::LoopBrowser;
    app.loop_browser.state.help_overlay = Some(crate::tui::loop_browser::LoopBrowserPane::Tree);
    assert!(!app.try_open_screen_switch_menu(ctrl_g()));
}

#[test]
fn menu_switches_directly_between_tui_primary_screens() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::LoopBrowser, None);
    assert_eq!(app.active_screen, PrimaryScreen::LoopBrowser);
    assert_eq!(
        app.active_screen,
        crate::screen_switch::PrimaryScreen::LoopBrowser
    );

    app.loop_browser.state.starting = false;
    app.switch_to_primary_screen(PrimaryScreen::Keyboard, None);
    assert_eq!(app.active_screen, PrimaryScreen::Keyboard);
    assert_eq!(
        app.active_screen,
        crate::screen_switch::PrimaryScreen::Keyboard
    );

    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);
    assert_eq!(app.active_screen, PrimaryScreen::Notepad);
    assert_eq!(app.notepad.mode, Mode::Normal);
}

#[test]
fn selecting_current_screen_only_closes_menu() {
    let mut app = TuiApp::new_for_test(test_config());
    app.screen_switch_menu.open();
    assert_eq!(
        app.handle_screen_switch_menu_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE,)),
        None
    );
    assert_eq!(app.active_screen, PrimaryScreen::Notepad);
    assert!(!app.screen_switch_menu.is_open());
}

#[test]
fn external_screen_switch_closes_an_open_menu() {
    let mut app = TuiApp::new_for_test(test_config());
    app.screen_switch_menu.open();

    app.switch_to_primary_screen(PrimaryScreen::Daw, None);

    assert_eq!(app.active_screen, PrimaryScreen::Daw);
    assert!(!app.screen_switch_menu.is_open());
}
