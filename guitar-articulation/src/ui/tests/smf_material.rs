//! SMF 素材の描画: 読み込み overlay、MML 欄の `SMF: <file 名>`、見出しの [SMF]・[top]、下段のキー。

use std::path::PathBuf;

use super::*;
use cmrt_tui_core::theme::MONOKAI_GRAY;

fn shift(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT)
}

fn at(seconds: f64, message: [u8; 3]) -> TimedMidiEvent {
    TimedMidiEvent { seconds, message }
}

/// 和音 C4/E4/G4（0〜1）と、メロディ A4（1〜2）。
fn chord_then_melody() -> Vec<TimedMidiEvent> {
    vec![
        at(0.0, [0x90, 60, 100]),
        at(0.0, [0x90, 64, 100]),
        at(0.0, [0x90, 67, 100]),
        at(1.0, [0x80, 60, 0]),
        at(1.0, [0x80, 64, 0]),
        at(1.0, [0x80, 67, 0]),
        at(1.0, [0x90, 69, 100]),
        at(2.0, [0x80, 69, 0]),
    ]
}

fn screen_with_smf() -> GuitarArticulationScreen {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(shift('O'));
    screen.load_smf(
        PathBuf::from("dir").join("song.mid"),
        Ok(chord_then_melody()),
    );
    screen
}

fn top(buffer: &Buffer) -> String {
    squeezed(&rows_in(buffer, Rect::new(0, 0, WIDTH, 1)))
}

#[test]
fn shift_o_draws_the_input_overlay_with_its_keys_and_puts_the_cursor_in_it() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(shift('O'));
    for ch in "a.mid".chars() {
        screen.handle_key_event(key(KeyCode::Char(ch)));
    }

    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal.draw(|f| draw(&screen, f)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let area = crate::ui::smf_material::input_rect(buffer.area);
    let overlay = squeezed(&rows_in(&buffer, area));
    let title: String = crate::ui::smf_material::INPUT_TITLE
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    assert!(overlay.contains(&title), "{overlay}");
    assert!(overlay.contains("a.mid"), "{overlay}");
    let cursor = terminal.get_cursor_position().unwrap();
    assert_eq!(cursor.y, area.y + 1);
    // `a.mid` の 5 文字の後ろ。
    assert_eq!(cursor.x, area.x + 1 + 5);
}

#[test]
fn the_closed_input_is_not_drawn() {
    let screen = screen_with_mml("o3 l8 e g a");
    let buffer = render(&screen);
    let all = squeezed(&rows_in(&buffer, buffer.area));
    assert!(!all.contains("SMFを読む"), "{all}");
}

#[test]
fn the_mml_field_shows_the_smf_file_name_in_grey_instead_of_the_mml() {
    let screen = screen_with_smf();
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    let rows = rows_in(&buffer, layout.input);
    let field = squeezed(&rows);
    assert!(field.contains("SMF:song.mid"), "{field}");
    assert!(!field.contains("o3l8ega"), "{field}");
    let (dy, row) = rows
        .iter()
        .enumerate()
        .find(|(_, row)| row.contains("SMF:"))
        .unwrap();
    let dx = row[..row.find("SMF:").unwrap()].chars().count() as u16;
    let cell = buffer
        .cell((layout.input.x + dx, layout.input.y + dy as u16))
        .unwrap();
    assert_eq!(cell.fg, MONOKAI_GRAY);
}

#[test]
fn the_title_shows_smf_and_top_while_they_are_on() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    assert!(!top(&render(&screen)).contains("[SMF]"));

    screen.handle_key_event(shift('O'));
    screen.load_smf(PathBuf::from("song.mid"), Ok(chord_then_melody()));
    let title = top(&render(&screen));
    assert!(title.contains("GuitarArticulation[SMF]"), "{title}");
    assert!(!title.contains("[top]"), "{title}");

    screen.handle_key_event(shift('M'));
    let title = top(&render(&screen));
    assert!(title.contains("[SMF][top]"), "{title}");

    // `i` で MML を確定すると MML 素材へ戻り、どちらも消える。
    screen.handle_key_event(key(KeyCode::Char('i')));
    screen.handle_key_event(key(KeyCode::Enter));
    let buffer = render(&screen);
    let title = top(&buffer);
    assert!(!title.contains("[SMF]"), "{title}");
    assert!(!title.contains("[top]"), "{title}");
    let field = squeezed(&rows_in(&buffer, layout_for(buffer.area, &screen).input));
    assert!(field.contains("o3l8ega"), "{field}");
}

#[test]
fn the_status_line_names_o_and_while_on_smf_names_m_and_both_reach_help() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    let status = |screen: &GuitarArticulationScreen| {
        let buffer = render(screen);
        squeezed(&rows_in(&buffer, layout_for(buffer.area, screen).status))
    };
    let mml = status(&screen);
    assert!(mml.contains("O:SMF"), "{mml}");
    assert!(mml.ends_with("?:help"), "{mml}");

    screen.handle_key_event(shift('O'));
    screen.load_smf(PathBuf::from("song.mid"), Ok(chord_then_melody()));
    let smf = status(&screen);
    assert!(smf.contains("M:top"), "{smf}");
    assert!(!smf.contains("H:履歴"), "{smf}");
    assert!(smf.ends_with("?:help"), "{smf}");

    // 幅 100 の端末の枠の内側（98 桁）に収まる。
    for text in [KEYBIND_TEXT, SMF_KEYBIND_TEXT] {
        assert!(Line::from(text).width() <= usize::from(WIDTH - 2), "{text}");
    }
}
