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

/// loop browser の `/` 絞り込み入力中は、すべてのキーが入力欄へ入る。
/// Ctrl+G も例外ではない（notepad が `Mode::Normal` のときだけ開く形に揃えた）。
#[test]
fn the_loop_tree_filter_input_blocks_the_screen_switch_menu() {
    let mut app = TuiApp::new_for_test(test_config());
    app.begin_loop_browser_startup();
    app.loop_browser.state.starting = false;
    assert!(app.can_open_screen_switch_menu());

    app.loop_browser.state.handle_key(KeyCode::Char('/'));
    assert!(!app.try_open_screen_switch_menu(ctrl_g()));
    assert!(!app.screen_switch_menu.is_open());

    // Enter で確定すれば（絞り込みは効いたまま）また開く。
    app.loop_browser.state.handle_key(KeyCode::Enter);
    assert!(app.try_open_screen_switch_menu(ctrl_g()));
    assert!(app.screen_switch_menu.is_open());

    app.screen_switch_menu
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.stop_loop_browser();
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
fn entering_grid_sequencer_does_not_relock_the_shared_patch_catalog() {
    let mut app = TuiApp::new_for_test(test_config());
    *app.patch_load_state.lock().unwrap() =
        PatchLoadState::ready(make_patches(&["patches_factory/Polysynths/Test.fxp"]));

    app.switch_to_primary_screen(PrimaryScreen::GridSequencer, None);

    assert_eq!(app.active_screen, PrimaryScreen::GridSequencer);
    assert!(app.grid_sequencer.state.is_running());
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

/// keyboard の `M`（plugin solo/mute）を開いている間は、キーが overlay へ入るので Ctrl+G を塞ぐ。
#[test]
fn the_keyboard_plugin_menu_blocks_the_screen_switch_menu() {
    let mut app = TuiApp::new_for_test(test_config());
    app.start_keyboard(None);

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('M'), KeyModifiers::SHIFT));
    assert!(app.keyboard.plugin_menu().is_some());
    assert!(!app.try_open_screen_switch_menu(ctrl_g()));

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.try_open_screen_switch_menu(ctrl_g()));
    app.screen_switch_menu
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.finish_keyboard();
}

#[test]
fn the_keyboard_effect_add_overlay_blocks_the_screen_switch_menu() {
    let mut app = TuiApp::new_for_test(test_config());
    // keyboard だけ effect の catalog を持たせ直す。
    app.keyboard = std::mem::replace(
        &mut app.keyboard,
        crate::tui::keyboard::KeyboardScreen::new(
            None,
            crate::tui::keyboard::KeyboardState::from_session(Default::default()),
            Default::default(),
            crate::tui::keyboard::KeyboardNoteGuide::new(None),
        ),
    )
    .with_effect_plugins(super::chord_chart_auto_reverb::effect_plugins());
    app.start_keyboard(None);

    for ch in ['l', 'a'] {
        app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
    }
    assert!(app.keyboard.effect_pane().is_adding());
    assert!(!app.try_open_screen_switch_menu(ctrl_g()));

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!app.keyboard.effect_pane().is_adding());
    assert!(app.try_open_screen_switch_menu(ctrl_g()));
    app.screen_switch_menu
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.finish_keyboard();
}

#[test]
fn the_keyboard_help_blocks_shared_overlays_until_closed() {
    let mut app = TuiApp::new_for_test(test_config());
    app.start_keyboard(None);
    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
    assert!(app.keyboard.help_open());
    assert!(!app.can_open_screen_switch_menu());
    assert!(!app.try_open_screen_switch_menu(ctrl_g()));
    assert!(!app.screen_switch_menu.is_open());
    let ctrl_p = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    assert!(!app.try_open_mml_overlay(ctrl_p));
    assert!(!app.mml_overlay.is_open());
    assert!(!app.keyboard.periodic_sending_stopped());

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.try_open_screen_switch_menu(ctrl_g()));
    app.screen_switch_menu
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.try_open_mml_overlay(ctrl_p));
    assert!(app.mml_overlay.is_open());
    app.handle_mml_overlay_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.finish_keyboard();
}
