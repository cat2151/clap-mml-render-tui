//! auto reverb を扱わない selector（Chord Chart の `t`・grid の PATCH 欄）には、表示行が出ない。

use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::screen_switch::PrimaryScreen;

const WIDTH: u16 = 160;
const HEIGHT: u16 = 45;

fn ready_patches(displays: &[&str]) -> crate::tui::PatchLoadState {
    crate::tui::PatchLoadState::ready(
        displays
            .iter()
            .map(|display| (display.to_string(), display.to_lowercase()))
            .collect(),
    )
}

fn screen(app: &mut TuiApp<'static>) -> String {
    render_lines(app, WIDTH, HEIGHT).join("\n")
}

#[test]
fn the_chord_chart_t_selector_has_no_auto_reverb_row() {
    let (_tmp, _env_guard) = cmrt_history::test_support::temp_local_dirs("chord_auto_reverb_row");
    let mut app = TuiApp::new_for_test(test_config());
    app.chord_chart.set_bass_enabled(false);
    *app.patch_load_state.lock().unwrap() = ready_patches(&["Pads/Warm Pad.fxp"]);
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    assert!(app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)));

    let screen = screen(&mut app);
    assert!(screen.contains("Pads/Warm Pad.fxp"), "{screen}");
    assert!(!screen.contains("auto reverb"), "{screen}");
}

#[test]
fn the_grid_patch_selector_has_no_auto_reverb_row() {
    let mut app = TuiApp::new_for_test(test_config());
    *app.patch_load_state.lock().unwrap() = ready_patches(&["Keys/Alpha.fxp", "Keys/Beta.fxp"]);
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
    assert!(!screen.contains("auto reverb"), "{screen}");
}
