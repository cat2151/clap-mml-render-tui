//! Chord Chart 画面と共有ランタイムの接続（`chord_chart_glue` / `runtime::screen`）。
//!
//! 画面そのものの挙動は `cmrt-chord-chart` crate 側のテストが見る。ここで見るのは
//! 「app に繋がっているか」だけ。

use super::*;
use crate::screen_switch::PrimaryScreen;

fn ctrl_g() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL)
}

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn switching_to_the_chord_chart_and_back_leaves_the_other_screens_alone() {
    let mut app = TuiApp::new_for_test(test_config());

    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    assert_eq!(app.active_screen, PrimaryScreen::ChordChart);
    // 音を鳴らさない画面なので、入っても何も再生が始まらない。
    assert!(!app.grid_sequencer.state.is_running());

    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);
    assert_eq!(app.active_screen, PrimaryScreen::Notepad);
}

/// 曲は画面をまたいでも保持される（load は起動時の 1 回だけ）。
#[test]
fn returning_to_the_chord_chart_keeps_the_song() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    app.chord_chart.song.push_section("B", "IIm-V-I-VIm");

    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    assert_eq!(app.chord_chart.song.sections.len(), 2);
}

#[test]
fn ctrl_g_opens_the_menu_from_the_chord_chart_unless_the_help_is_open() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    assert!(app.try_open_screen_switch_menu(ctrl_g()));

    app.screen_switch_menu
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.chord_chart.help_open = true;
    assert!(!app.try_open_screen_switch_menu(ctrl_g()));
}

/// 人間の手順（`Ctrl+G` → `C`）をそのまま辿る。menu の綴りを間違えても
/// `switch_to_primary_screen` 単体のテストは緑のままなので、ここで閉じておく。
#[test]
fn ctrl_g_then_c_opens_the_chord_chart() {
    let mut app = TuiApp::new_for_test(test_config());
    assert!(app.try_open_screen_switch_menu(ctrl_g()));

    let target = app.handle_screen_switch_menu_key(plain(KeyCode::Char('c')));

    assert_eq!(target, Some(PrimaryScreen::ChordChart));
    app.switch_to_primary_screen(target.unwrap(), None);
    assert_eq!(app.active_screen, PrimaryScreen::ChordChart);
    assert!(!app.screen_switch_menu.is_open());
}

/// 音を鳴らさずマウスも使わない画面。入力欄を開いていない間は notepad と同じまま。
#[test]
fn the_chord_chart_asks_for_neither_mouse_capture_nor_a_textarea_cursor() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    assert!(!app.uses_mouse_capture());
    assert!(!app.uses_textarea_cursor());
}

/// `i` / `n` の 1 行入力欄を開いている間だけ、端末カーソルを入力欄の形にする。
/// ここが false のままだと「打っている場所」が画面に出ない。
#[test]
fn the_textarea_cursor_follows_the_line_input() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    app.handle_chord_chart_key_event(plain(KeyCode::Char('i')));
    assert!(app.chord_chart.line_input_open());
    assert!(app.uses_textarea_cursor());
    // マウスは相変わらず要らない。
    assert!(!app.uses_mouse_capture());

    app.handle_chord_chart_key_event(plain(KeyCode::Esc));
    assert!(!app.uses_textarea_cursor());
}

/// 入力中の `Ctrl+G` で画面が飛ぶと、打ちかけの文字列が消える。
#[test]
fn ctrl_g_does_not_leave_the_screen_while_the_line_input_is_open() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    app.handle_chord_chart_key_event(plain(KeyCode::Char('n')));

    assert!(!app.try_open_screen_switch_menu(ctrl_g()));

    app.handle_chord_chart_key_event(plain(KeyCode::Esc));
    assert!(app.try_open_screen_switch_menu(ctrl_g()));
}

#[test]
fn keys_reach_the_screen_through_the_glue() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    app.chord_chart.song.push_section("B", "IIm-V-I-VIm");

    app.handle_chord_chart_key_event(plain(KeyCode::Char('j')));
    assert_eq!(app.chord_chart.clamped_section_cursor(), 1);

    app.handle_chord_chart_key_event(plain(KeyCode::Char('l')));
    assert_eq!(app.chord_chart.focus, cmrt_chord_chart::Pane::Arrangement);

    app.handle_chord_chart_key_event(plain(KeyCode::Char('?')));
    assert!(app.chord_chart.help_open);
}

/// `q` は glue を抜けてランタイムまで届く（ここで握り潰されるとアプリが終了しない）。
#[test]
fn q_reaches_the_glue_as_a_quit() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    assert_eq!(
        app.handle_chord_chart_key_event(plain(KeyCode::Char('q'))),
        cmrt_chord_chart::ChordChartAction::Quit
    );
    // 移動キーは終了にならない（`q` だけが Quit）。
    assert_eq!(
        app.handle_chord_chart_key_event(plain(KeyCode::PageDown)),
        cmrt_chord_chart::ChordChartAction::Continue
    );
}

/// app の描画振り分けが chord chart を呼んでいるか（画面ごとの絵の中身は crate 側で見る）。
#[test]
fn the_root_draw_dispatches_to_the_chord_chart_screen() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    let lines = crate::tui::ui::tests::render_lines(&mut app, 80, 24);
    let rendered = lines.join("\n");

    // どちらも ASCII なので、全角のセル 2 個問題（crate 側テストの `squeeze`）は起きない。
    assert!(rendered.contains("Chord Chart"), "{rendered}");
    assert!(rendered.contains("I-V-VIm-IV"), "{rendered}");
}
