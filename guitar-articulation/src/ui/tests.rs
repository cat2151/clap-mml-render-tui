use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};

use super::*;
use crate::TimedMidiEvent;

mod arp_overlay;
mod column_rule_rows;
mod event_list;
mod help;
mod history_overlay;
mod humanize_row;
mod ineffective_notice;
mod instrument_title;
mod long_material;
mod playhead_event_list;
mod playhead_row;
mod rule_groups;
mod rule_lanes;
mod rule_list_filter;
mod rule_list_overlay;
mod sample_midi;
mod smf_material;
mod title_flags;

const WIDTH: u16 = 100;
const HEIGHT: u16 = 56;

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
    for name in ["G2", "F#2", "E2", "s:autohammer/pull", "t:KS"] {
        assert!(matrix.contains(name), "{name} が matrix に無い: {matrix}");
    }
    assert!(!matrix.contains("F2"), "音の無い段は出さない: {matrix}");

    let plain = squeezed(&rows_in(&buffer, layout.plain));
    let converted = squeezed(&rows_in(&buffer, layout.converted));
    for name in ["E2", "F#2", "G2"] {
        assert!(plain.contains(name), "rawに {name} が無い: {plain}");
    }
    assert!(converted.contains("KSSus_Down"), "{converted}");
    // Articulated は頭に列 CC の既定値が積まれるので、カーソル列の音を 1 列ずつ見る。
    let mut screen = screen;
    for name in ["E2", "F#2", "G2"] {
        let converted = squeezed(&rows_in(&render(&screen), layout.converted));
        assert!(
            converted.contains(name),
            "Articulatedに {name} が無い: {converted}"
        );
        screen.handle_key_event(key(KeyCode::Char('l')));
    }
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
fn event_rows_name_the_keyswitch_only_where_asked() {
    let event = TimedMidiEvent {
        seconds: 0.25,
        message: [0x90, 26, 127],
    };

    assert_eq!(
        EventRow::new(&event, true).text(12),
        "  0.250 on  KS Hammer-On     127"
    );
    assert_eq!(
        EventRow::new(&event, false).text(12),
        "  0.250 on  D1               127"
    );
}

#[test]
fn event_rows_name_the_control_and_show_note_off_as_off() {
    let cc = TimedMidiEvent {
        seconds: 0.5,
        message: [0xB0, 30, 75],
    };
    let off = TimedMidiEvent {
        seconds: 0.5,
        message: [0x80, 40, 0],
    };

    assert_eq!(
        EventRow::new(&cc, true).text(13),
        "  0.500 cc  picking noise      75"
    );
    assert_eq!(
        EventRow::new(&off, true).text(13),
        "  0.500 off E2                off"
    );
}

#[test]
fn event_rows_name_the_release_type_and_volume() {
    let shape = TimedMidiEvent {
        seconds: 0.5,
        message: [0xB0, 24, 20],
    };
    let level = TimedMidiEvent {
        seconds: 0.5,
        message: [0xB0, 25, 100],
    };

    assert_eq!(
        EventRow::new(&shape, true).text(14),
        "  0.500 cc  release type       Hard 20"
    );
    assert_eq!(
        EventRow::new(&level, true).text(14),
        "  0.500 cc  release volume     100"
    );
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

    let marks = mark_cells(&buffer, layout.matrix, "a:hammer/pull");
    assert_eq!(marks.len(), 1, "{marks:?}");
    assert_eq!(buffer.cell(marks[0]).unwrap().symbol(), "a");
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
    assert!(status.contains("h/l:移動"), "{status}");
    assert!(status.contains("a:H/P"), "{status}");
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
fn auto_row_sits_above_the_ks_row_and_marks_picks_and_legatos() {
    // a+ で 1 本の弦の幅（5 半音）を超え、次の弦をピッキングする。
    let mut screen = screen_with_mml("o3 l8 e f+ g a+");
    screen.handle_key_event(key(KeyCode::Char('s')));
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    let rows = rows_in(&buffer, layout.matrix);
    let auto = rows
        .iter()
        .position(|r| r.contains("s:auto hammer/pull"))
        .unwrap();
    let ks = rows.iter().position(|r| r.contains("t:KS")).unwrap();
    assert_eq!(auto + 1, ks, "{rows:#?}");
    let marks = row_marks(&buffer, layout.matrix, "s:auto hammer/pull");
    assert_eq!(marks, "D●●U", "{rows:#?}");
}

#[test]
fn economy_row_sits_on_top_of_the_rule_rows_and_shows_the_strokes() {
    let mut screen = screen_with_mml("o3 l8 e f+ g f+");
    screen.handle_key_event(key(KeyCode::Char('e')));
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    let rows = rows_in(&buffer, layout.matrix);
    let eco = rows
        .iter()
        .position(|r| r.contains("e:economy picking"))
        .unwrap();
    let auto = rows
        .iter()
        .position(|r| r.contains("s:auto hammer/pull"))
        .unwrap();
    assert_eq!(eco + 1, auto, "{rows:#?}");
    let marks = row_marks(&buffer, layout.matrix, "e:economy picking");
    // 4 音目は次の弦で下行なので、スイープせずオルタネイトのまま U。
    assert_eq!(marks, "DUDU", "{rows:#?}");
}

#[test]
fn the_economy_row_shows_its_pull_offs_and_never_shares_a_column_with_the_auto_row() {
    // 低い弦へ移る前の音（f）がプリングになる。
    let mut screen = screen_with_mml("l16 a g f e d c");
    screen.handle_key_event(key(KeyCode::Char('e')));
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);
    assert_eq!(
        row_marks(&buffer, layout.matrix, "e:economy picking"),
        "DUPDUD"
    );
    assert_eq!(row_marks(&buffer, layout.matrix, "s:auto hammer/pull"), "");

    screen.handle_key_event(key(KeyCode::Char('s')));
    let buffer = render(&screen);
    assert_eq!(row_marks(&buffer, layout.matrix, "e:economy picking"), "");
    assert_eq!(
        row_marks(&buffer, layout.matrix, "s:auto hammer/pull"),
        "D●●U●D"
    );
}

#[test]
fn the_committed_effect_chain_follows_the_instrument_in_the_title() {
    let mut screen =
        GuitarArticulationScreen::with_effect_plugins(crate::test_effects::amp_plugins());
    screen.enter();
    screen.handle_key_event(key(KeyCode::Esc));
    let top = |screen: &GuitarArticulationScreen| {
        squeezed(&rows_in(&render(screen), Rect::new(0, 0, WIDTH, 1)))
    };
    assert!(
        top(&screen).contains("METAL-GTXLite[起動:"),
        "{}",
        top(&screen)
    );

    for code in [
        KeyCode::Char('x'),
        KeyCode::Char('a'),
        KeyCode::Enter,
        KeyCode::Enter,
    ] {
        screen.handle_key_event(key(code));
    }

    assert!(
        top(&screen).contains("METAL-GTXLite→TestAmp:Clean"),
        "{}",
        top(&screen)
    );
}

#[test]
fn x_draws_the_effect_chain_overlay_over_the_screen() {
    let mut screen =
        GuitarArticulationScreen::with_effect_plugins(crate::test_effects::amp_plugins());
    screen.enter();
    screen.handle_key_event(key(KeyCode::Esc));

    screen.handle_key_event(key(KeyCode::Char('x')));
    let chain = squeezed(&rows_in(&render(&screen), Rect::new(0, 0, WIDTH, HEIGHT)));
    assert!(chain.contains("EFFECTCHAIN"), "{chain}");
    assert!(chain.contains("instrument:METAL-GTXLite"), "{chain}");

    screen.handle_key_event(key(KeyCode::Char('a')));
    let add = squeezed(&rows_in(&render(&screen), Rect::new(0, 0, WIDTH, HEIGHT)));
    assert!(add.contains("AmpSimulator"), "{add}");
}

#[test]
fn the_note_preview_shows_in_the_title_and_n_in_the_keybinds() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    let top = |screen: &GuitarArticulationScreen| {
        squeezed(&rows_in(&render(screen), Rect::new(0, 0, WIDTH, 1)))
    };
    assert!(!top(&screen).contains("[1音]"), "{}", top(&screen));
    let buffer = render(&screen);
    let status = squeezed(&rows_in(&buffer, layout_for(buffer.area, &screen).status));
    assert!(status.contains("n:1音"), "{status}");

    screen.handle_key_event(key(KeyCode::Char('n')));
    assert!(top(&screen).contains("[1音]"), "{}", top(&screen));
}

/// 枠の見出しや下の枠の行。効いていないルールの知らせが段の見出しと同じ文字列を含むので、段を探すときは飛ばす。
fn is_border_row(row: &str) -> bool {
    row.contains('┌') || row.contains('└')
}

/// 見出しが `label` の段の記号（空白と枠を落とす）。
fn row_marks(buffer: &Buffer, area: Rect, label: &str) -> String {
    let rows = rows_in(buffer, area);
    let row = rows
        .iter()
        .find(|r| !is_border_row(r) && r.contains(label))
        .unwrap_or_else(|| panic!("{label} の段が無い: {rows:#?}"));
    row[row.find(label).unwrap() + label.len()..]
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '│')
        .collect()
}

/// 見出しが `label` の段で、記号の在るセル（空白と枠を除く）の `(x, y)`。
fn mark_cells(buffer: &Buffer, area: Rect, label: &str) -> Vec<(u16, u16)> {
    let rows = rows_in(buffer, area);
    let (offset, row) = rows
        .iter()
        .enumerate()
        .find(|(_, r)| !is_border_row(r) && r.contains(label))
        .unwrap_or_else(|| panic!("{label} の段が無い: {rows:#?}"));
    let start = row[..row.find(label).unwrap() + label.len()]
        .chars()
        .count() as u16;
    let y = area.y + offset as u16;
    (area.x + start..area.x + area.width)
        .filter(|&x| {
            let symbol = buffer.cell((x, y)).unwrap().symbol();
            !symbol.trim().is_empty() && symbol != "│"
        })
        .map(|x| (x, y))
        .collect()
}

/// `area` の中で `label` が始まるセルの文字色。
fn label_fg(buffer: &Buffer, area: Rect, label: &str) -> ratatui::style::Color {
    let rows = rows_in(buffer, area);
    let (offset, row) = rows
        .iter()
        .enumerate()
        .find(|(_, row)| !is_border_row(row) && row.contains(label))
        .unwrap_or_else(|| panic!("{label} の行が無い: {rows:#?}"));
    let column = row[..row.find(label).unwrap()].chars().count() as u16;
    buffer
        .cell((area.x + column, area.y + offset as u16))
        .unwrap()
        .fg
}
