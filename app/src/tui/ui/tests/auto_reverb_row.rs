//! selector の auto reverb の表示行。grid の PATCH 欄にも Chord Chart の `t` にも出る。

use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::screen_switch::PrimaryScreen;
use crate::tui::tests::chord_chart_auto_reverb::{effect_plugins, patch_load, DEXED_SNARE};

const WIDTH: u16 = 160;
const HEIGHT: u16 = 45;

fn with_catalogs(app: &mut TuiApp<'static>) {
    app.effect_plugins = effect_plugins();
    *app.patch_load_state.lock().unwrap() = patch_load();
}

fn screen(app: &mut TuiApp<'static>) -> String {
    render_lines(app, WIDTH, HEIGHT).join("\n")
}

#[test]
fn the_chord_chart_t_selector_shows_the_auto_reverb_row() {
    let (_tmp, _env_guard) = cmrt_history::test_support::temp_local_dirs("chord_auto_reverb_row");
    let mut app = TuiApp::new_for_test(test_config());
    app.chord_chart.set_bass_enabled(false);
    with_catalogs(&mut app);
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    assert!(app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)));

    let screen = screen(&mut app);
    assert!(screen.contains("auto reverb:"), "{screen}");
}

#[test]
fn the_grid_patch_selector_shows_the_auto_reverb_row() {
    let (_tmp, _env_guard) = cmrt_history::test_support::temp_local_dirs("grid_auto_reverb_row");
    let mut app = TuiApp::new_for_test(test_config());
    with_catalogs(&mut app);
    app.active_screen = PrimaryScreen::GridSequencer;
    let buffer = render_buffer(&mut app, WIDTH, HEIGHT);
    let (patch_x, header_y) = find_text(&buffer, "PATCH");

    app.handle_grid_sequencer_mouse_event(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: patch_x,
            row: header_y + 1,
            modifiers: KeyModifiers::NONE,
        },
        Rect::new(0, 0, WIDTH, HEIGHT),
    );

    let screen = screen(&mut app);
    assert!(screen.contains("instance 1 patch select"), "{screen}");
    assert!(screen.contains("auto reverb:"), "{screen}");
}
