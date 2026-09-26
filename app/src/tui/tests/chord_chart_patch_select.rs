//! Chord Chart の `t` / `Shift+T` は、MML 入力欄を開かずに音色 selector だけを重ねる。

use super::chord_chart_preview::app_on_the_chord_chart;
use super::*;
use crate::tui::ui::tests::{render_buffer, render_lines};
use cmrt_tui_core::buffer_test::find_text_ignoring_spaces;

/// 入力欄の枠のタイトル（` CHORD → Chord Chart [音色] `）の ASCII 部分。
const INPUT_BOX_TITLE: &str = "Chord Chart [";

/// 入力欄の枠がどの syntax のタイトルでも描かれていない（` MML [音色] ` も含む）。
fn assert_no_input_box(screen: &str) {
    for title in [INPUT_BOX_TITLE, "MML ["] {
        assert!(
            !screen.contains(title),
            "入力欄の枠が描かれていないはず（{title}）:\n{screen}"
        );
    }
}

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn app_with_patches(state: PatchLoadState) -> TuiApp<'static> {
    let app = app_on_the_chord_chart();
    *app.patch_load_state.lock().unwrap() = state;
    app
}

fn ready() -> PatchLoadState {
    PatchLoadState::ready(make_patches(&["Basses/Bass 1.fxp", "Pads/Warm Pad.fxp"]))
}

#[test]
fn t_draws_the_patch_selector_without_the_mml_input_box() {
    let mut app = app_with_patches(ready());

    assert!(app.try_open_mml_overlay(plain(KeyCode::Char('t'))));

    assert!(!app.mml_overlay.is_open());
    assert!(app
        .chord_chart_patch_select
        .as_ref()
        .is_some_and(|(_, select)| select.is_select_open()));
    let screen = render_lines(&mut app, 120, 40).join("\n");
    assert!(
        screen.contains("Warm Pad"),
        "音色 selector が描かれているはず:\n{screen}"
    );
    assert_no_input_box(&screen);
}

/// 上の否定の assert が、入力欄を描いたときに実際に当たることの対照。
#[test]
fn the_degrees_editor_does_draw_the_mml_input_box() {
    let mut app = app_with_patches(ready());
    let section_id = app.chord_chart.selected_section().unwrap().id;
    app.open_chord_chart_degrees_overlay(section_id);

    let screen = render_lines(&mut app, 120, 40).join("\n");

    assert!(screen.contains(INPUT_BOX_TITLE), "{screen}");
}

#[test]
fn a_failed_catalog_shows_the_reason_until_esc() {
    let mut app = app_with_patches(PatchLoadState::Err("catalog failed".to_string()));

    assert!(app.try_open_mml_overlay(plain(KeyCode::Char('t'))));

    assert!(!app.mml_overlay.is_open());
    let screen = render_lines(&mut app, 120, 40).join("\n");
    assert!(screen.contains("catalog failed"), "{screen}");
    assert_no_input_box(&screen);

    app.handle_chord_chart_patch_select_key_event(plain(KeyCode::Esc));
    assert!(app.chord_chart_patch_select.is_none());
}

#[test]
fn a_loading_catalog_shows_the_waiting_notice_and_opens_once_ready() {
    let mut app = app_with_patches(PatchLoadState::Loading);

    assert!(app.try_open_mml_overlay(plain(KeyCode::Char('t'))));
    assert_no_input_box(&render_lines(&mut app, 120, 40).join("\n"));
    find_text_ignoring_spaces(&render_buffer(&mut app, 120, 40), "音色一覧を読み込み中");

    *app.patch_load_state.lock().unwrap() = ready();
    app.pump_mml_overlay();

    assert!(app
        .chord_chart_patch_select
        .as_ref()
        .is_some_and(|(_, select)| select.is_select_open()));
}
