use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};

use super::*;

const WIDTH: u16 = 100;
const HEIGHT: u16 = 30;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn screen_with_mml(mml: &str) -> GuitarArticulationScreen {
    let mut screen = GuitarArticulationScreen::default();
    screen.handle_key_event(key(KeyCode::Char('i')));
    for ch in mml.chars() {
        screen.handle_key_event(key(KeyCode::Char(ch)));
    }
    screen.handle_key_event(key(KeyCode::Enter));
    screen
}

fn render(screen: &GuitarArticulationScreen) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal.draw(|f| draw(screen, f)).unwrap();
    terminal.backend().buffer().clone()
}

/// 矩形の中の行（枠を含む）。
fn rows_in(buffer: &Buffer, area: Rect) -> Vec<String> {
    (area.y..area.y + area.height)
        .map(|y| {
            (area.x..area.x + area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
                .collect()
        })
        .collect()
}

/// 空白を落とした 1 本の文字列。全角のセル 2 つ目が空白で読めるのを吸収する。
fn squeezed(rows: &[String]) -> String {
    rows.concat()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

/// `(x, y)` ごとの音の升。
fn note_cells(buffer: &Buffer, area: Rect) -> Vec<(u16, u16)> {
    let mut out = Vec::new();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if buffer.cell((x, y)).unwrap().symbol() == "■" {
                out.push((x, y));
            }
        }
    }
    out
}

#[test]
fn three_rising_notes_fill_three_columns_on_three_rows_and_both_event_lists() {
    // `o3 e` は 40（E2）。matrix は使われている E2・F#2・G2 の 3 段だけで、音の無い F2 の段は出ない。
    let screen = screen_with_mml("o3 l8 e f+ g");
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    let cells = note_cells(&buffer, layout.matrix);
    assert_eq!(cells.len(), 3, "{cells:?}");
    let mut xs: Vec<u16> = cells.iter().map(|(x, _)| *x).collect();
    let mut ys: Vec<u16> = cells.iter().map(|(_, y)| *y).collect();
    xs.dedup();
    ys.sort();
    ys.dedup();
    assert_eq!(xs.len(), 3, "3 列: {cells:?}");
    assert_eq!(ys.len(), 3, "3 段: {cells:?}");
    // 上行なので、上の段（高い音）ほど右の列。`cells` は上の段から並ぶ。
    assert!(
        cells.windows(2).all(|pair| pair[1].0 < pair[0].0),
        "{cells:?}"
    );
    let matrix = squeezed(&rows_in(&buffer, layout.matrix));
    for name in ["G2", "F#2", "E2", "s:auto", "a:H/P"] {
        assert!(matrix.contains(name), "{name} が matrix に無い: {matrix}");
    }
    assert!(!matrix.contains("F2"), "音の無い段は出さない: {matrix}");

    let plain = squeezed(&rows_in(&buffer, layout.plain));
    let converted = squeezed(&rows_in(&buffer, layout.converted));
    for name in ["E2", "F#2", "G2"] {
        assert!(plain.contains(name), "rawに {name} が無い: {plain}");
        assert!(
            converted.contains(name),
            "Articulatedに {name} が無い: {converted}"
        );
    }
    assert!(converted.contains("KSSus_Down"), "{converted}");
    assert!(!plain.contains("KS"), "rawに KS は無い: {plain}");
}

#[test]
fn an_empty_screen_says_how_to_start() {
    let screen = GuitarArticulationScreen::default();
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    assert!(squeezed(&rows_in(&buffer, layout.input)).contains("(iキーでMML入力開始)"));
    assert!(note_cells(&buffer, layout.matrix).is_empty());
}

#[test]
fn event_text_names_the_keyswitch_only_where_asked() {
    let event = TimedMidiEvent {
        seconds: 0.25,
        message: [0x90, 26, 127],
    };

    assert_eq!(event_text(&event, true), "  0.250 on  KS Hammer-On   127");
    assert_eq!(event_text(&event, false), "  0.250 on  D1             127");
}

#[test]
fn note_names_put_middle_c_in_octave_four() {
    assert_eq!(note_name(60), "C4");
    assert_eq!(note_name(40), "E2");
    assert_eq!(note_name(42), "F#2");
}

#[test]
fn a_toggled_rule_shows_under_its_column_and_the_converted_take_names_the_keyswitch() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('a')));
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    let marks: Vec<(u16, u16)> = (layout.matrix.y..layout.matrix.y + layout.matrix.height)
        .flat_map(|y| (layout.matrix.x..layout.matrix.x + layout.matrix.width).map(move |x| (x, y)))
        .filter(|&(x, y)| buffer.cell((x, y)).unwrap().symbol() == "●")
        .collect();
    assert_eq!(marks.len(), 1, "{marks:?}");
    // 2 列目（F#2）の音の升と同じ x。
    let f_sharp = note_cells(&buffer, layout.matrix)[1];
    assert_eq!(marks[0].0, f_sharp.0);
    assert!(buffer
        .cell(marks[0])
        .unwrap()
        .modifier
        .contains(ratatui::style::Modifier::REVERSED));

    let converted = squeezed(&rows_in(&buffer, layout.converted));
    assert!(converted.contains("KSHammer-On"), "{converted}");
    let status = squeezed(&rows_in(&buffer, layout.status));
    assert!(status.contains("h/l:列移動"), "{status}");
    assert!(status.contains("a:H/P切替"), "{status}");
}

/// 枠の左上の角の色。
fn corner_fg(buffer: &Buffer, area: Rect) -> ratatui::style::Color {
    buffer.cell((area.x, area.y)).unwrap().fg
}

/// キーを受ける側の枠だけが水色。全部灰色だと、端末が focus を失ったのと見分けがつかない。
#[test]
fn the_pane_that_takes_the_keys_has_the_cyan_border() {
    use cmrt_tui_core::theme::{MONOKAI_CYAN, MONOKAI_GRAY};
    let mut screen = screen_with_mml("o3 l8 e f+ g");

    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);
    assert_eq!(corner_fg(&buffer, layout.matrix), MONOKAI_CYAN);
    assert_eq!(corner_fg(&buffer, layout.input), MONOKAI_GRAY);

    screen.handle_key_event(key(KeyCode::Char('i')));
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);
    assert_eq!(corner_fg(&buffer, layout.matrix), MONOKAI_GRAY);
    assert_eq!(corner_fg(&buffer, layout.input), MONOKAI_CYAN);
}

#[test]
fn question_mark_draws_the_help_over_the_screen() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('?')));
    let buffer = render(&screen);

    let all = squeezed(&rows_in(&buffer, buffer.area));
    assert!(all.contains("ヘルプ(Keybinds)"), "{all}");
    assert!(all.contains("Hammer-On"), "{all}");
    let status = squeezed(&rows_in(&buffer, layout_for(buffer.area, &screen).status));
    assert!(status.contains("?:help"), "{status}");
}

#[test]
fn auto_row_sits_above_the_hammer_pull_row_and_marks_picks_and_legatos() {
    // a+ で 1 本の弦の幅（5 半音）を超え、次の弦をピッキングする。
    let mut screen = screen_with_mml("o3 l8 e f+ g a+");
    screen.handle_key_event(key(KeyCode::Char('s')));
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    let rows = rows_in(&buffer, layout.matrix);
    let auto = rows.iter().position(|r| r.contains("s:auto")).unwrap();
    let hammer = rows.iter().position(|r| r.contains("a:H/P")).unwrap();
    assert_eq!(auto + 1, hammer, "{rows:#?}");
    let marks: String = rows[auto]
        .chars()
        .skip_while(|ch| *ch != 'o')
        .skip(1)
        .filter(|ch| !ch.is_whitespace() && *ch != '│')
        .collect();
    assert_eq!(marks, "p●●p", "{rows:#?}");
}

#[test]
fn economy_row_sits_on_top_of_the_rule_rows_and_shows_the_strokes() {
    let mut screen = screen_with_mml("o3 l8 e f+ g f+");
    screen.handle_key_event(key(KeyCode::Char('e')));
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    let rows = rows_in(&buffer, layout.matrix);
    let eco = rows.iter().position(|r| r.contains("e:eco")).unwrap();
    let auto = rows.iter().position(|r| r.contains("s:auto")).unwrap();
    assert_eq!(eco + 1, auto, "{rows:#?}");
    let marks: String = rows[eco]
        .chars()
        .skip_while(|ch| *ch != 'o')
        .skip(1)
        .filter(|ch| !ch.is_whitespace() && *ch != '│')
        .collect();
    // 4 音目は次の弦で下行なので、スイープせずオルタネイトのまま U。
    assert_eq!(marks, "DUDU", "{rows:#?}");
}
