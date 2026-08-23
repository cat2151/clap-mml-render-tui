use super::*;
use crate::screen_switch::PrimaryScreen;

fn ctrl_p() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)
}

fn ctrl_t() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL)
}

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn ctrl_p_opens_the_overlay_on_each_primary_screen() {
    let mut app = TuiApp::new_for_test(test_config());
    assert!(app.try_open_mml_overlay(ctrl_p()));
    assert!(app.mml_overlay.is_open());

    app.handle_mml_overlay_key_event(press(KeyCode::Esc));
    app.start_keyboard(None);
    assert!(app.try_open_mml_overlay(ctrl_p()));

    app.handle_mml_overlay_key_event(press(KeyCode::Esc));
    app.begin_loop_browser_startup();
    app.loop_browser.state.starting = false;
    assert!(app.try_open_mml_overlay(ctrl_p()));

    app.handle_mml_overlay_key_event(press(KeyCode::Esc));
    app.enter_grid_sequencer();
    assert!(app.try_open_mml_overlay(ctrl_p()));
}

/// オーバーレイは音源インスタンスを借りるので、開いた時点で今の画面の演奏は止まる。
#[test]
fn opening_the_overlay_stops_the_grid_sequencer() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::GridSequencer, None);
    assert!(app.grid_sequencer.state.is_running());

    assert!(app.try_open_mml_overlay(ctrl_p()));

    assert!(!app.grid_sequencer.state.is_running());
    // 画面自体は grid sequencer のまま。オーバーレイはその上に重なるだけ。
    assert_eq!(app.active_screen, PrimaryScreen::GridSequencer);
}

/// 借りた音源は閉じるときに返す。止めた演奏はそのまま戻る。
#[test]
fn closing_the_overlay_resumes_the_grid_sequencer() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::GridSequencer, None);
    app.try_open_mml_overlay(ctrl_p());
    assert!(!app.grid_sequencer.is_playing());

    app.handle_mml_overlay_key_event(press(KeyCode::Esc));

    assert!(app.grid_sequencer.is_playing());
    assert_eq!(app.active_screen, PrimaryScreen::GridSequencer);
}

#[test]
fn closing_the_overlay_restarts_the_loop_browser() {
    let mut app = TuiApp::new_for_test(test_config());
    app.begin_loop_browser_startup();
    app.loop_browser.state.starting = false;
    app.try_open_mml_overlay(ctrl_p());

    app.handle_mml_overlay_key_event(press(KeyCode::Esc));

    assert!(app.loop_browser.state.starting);
}

#[test]
fn a_modal_screen_state_blocks_the_overlay() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::GridSequencer, None);
    app.grid_sequencer.help_open = true;

    assert!(!app.try_open_mml_overlay(ctrl_p()));
    assert!(!app.mml_overlay.is_open());
}

#[test]
fn other_keys_do_not_open_the_overlay() {
    let mut app = TuiApp::new_for_test(test_config());

    assert!(!app.try_open_mml_overlay(press(KeyCode::Char('p'))));
    assert!(!app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL)));
    assert!(!app.mml_overlay.is_open());
}

#[test]
fn patch_select_requested_during_initial_load_opens_when_the_catalog_is_ready() {
    let mut app = TuiApp::new_for_test(test_config());
    *app.patch_load_state.lock().unwrap() = PatchLoadState::Loading;
    assert!(app.try_open_mml_overlay(ctrl_p()));

    app.handle_mml_overlay_key_event(ctrl_t());

    assert!(!app.mml_overlay.is_patch_select_open());
    assert!(app.mml_overlay.is_waiting_for_patch_catalog());

    *app.patch_load_state.lock().unwrap() =
        PatchLoadState::ready(make_patches(&["Leads/Lead 1.fxp"]));
    app.pump_mml_overlay();

    assert!(app.mml_overlay.is_patch_select_open());
    assert!(!app.mml_overlay.is_waiting_for_patch_catalog());
}

#[test]
fn typed_keys_reach_the_overlay_and_esc_closes_it() {
    let mut app = TuiApp::new_for_test(test_config());
    app.try_open_mml_overlay(ctrl_p());

    for code in "cde".chars().map(KeyCode::Char) {
        app.handle_mml_overlay_key_event(press(code));
    }
    assert_eq!(app.mml_overlay.value(), "cde");
    assert_eq!(app.mml_overlay.sounding(), [64]);

    app.handle_mml_overlay_key_event(press(KeyCode::Esc));
    assert!(!app.mml_overlay.is_open());
    assert!(app.mml_overlay.sounding().is_empty());
}

#[test]
fn the_overlay_starts_empty_every_time_it_opens() {
    let mut app = TuiApp::new_for_test(test_config());
    app.try_open_mml_overlay(ctrl_p());
    app.handle_mml_overlay_key_event(press(KeyCode::Char('c')));
    app.handle_mml_overlay_key_event(press(KeyCode::Esc));

    app.try_open_mml_overlay(ctrl_p());
    assert_eq!(app.mml_overlay.value(), "");
}
