//! 奏法リスト overlay の並び（5 段・KS の 3 行折り返し・横ずらし）と強調。

use crossterm::event::KeyCode;
use ratatui::{backend::TestBackend, buffer::Buffer, style::Modifier, Terminal};

use super::{draw, is_border_row, key, render, rows_in, screen_with_mml, HEIGHT};
use crate::GuitarArticulationScreen;

/// 「なし」の 1 文字目。全角の 2 セル目は空白で読めるので、続けた文字列では探せない。
const NONE: &str = "な";

fn render_at_width(screen: &GuitarArticulationScreen, width: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, HEIGHT)).unwrap();
    terminal.draw(|f| draw(screen, f)).unwrap();
    terminal.backend().buffer().clone()
}

fn row_of(rows: &[String], label: &str) -> usize {
    rows.iter()
        .position(|row| row.contains(label))
        .unwrap_or_else(|| panic!("{label} の行が無い: {rows:#?}"))
}

/// overlay の中で `label` が始まるセルに反転（太字 + 背景）が掛かっているか。
/// matrix の見出しにも同じ文字列（`v:vibrato` など）があるので、overlay の枠から下を探す。
fn is_highlighted(buffer: &Buffer, label: &str) -> bool {
    let rows = rows_in(buffer, buffer.area);
    let top = row_of(&rows, "j/k:段");
    let y = top + row_of(&rows[top..], label);
    let x = rows[y][..rows[y].find(label).unwrap()].chars().count() as u16;
    let cell = buffer.cell((x, y as u16)).unwrap();
    cell.modifier.contains(Modifier::BOLD)
}

#[test]
fn the_ks_lane_wraps_by_group_and_the_other_lanes_follow_one_row_each() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('t')));
    let buffer = render_at_width(&screen, 80);
    let rows = rows_in(&buffer, buffer.area);

    let first = row_of(&rows, "a:hammer/pull");
    assert!(rows[first].contains(NONE), "{}", rows[first]);
    let order = [
        "a:hammer/pull",
        "o:slide up/down",
        "g:pick scratch",
        "v:vibrato",
        "f:long/extra",
        "i:power chord",
        "n:position rel",
    ];
    for (offset, label) in order.into_iter().enumerate() {
        assert_eq!(row_of(&rows[first..], label), offset, "{label}");
    }
    // 続きの行（ピッチ・飛び道具）には「なし」を置かない。
    assert!(!rows[first + 1].contains(NONE), "{}", rows[first + 1]);
    assert!(rows[first + 4].contains(NONE), "{}", rows[first + 4]);
    assert!(
        rows[first + 6].contains("s:auto slide out"),
        "{}",
        rows[first + 6]
    );
}

#[test]
fn a_row_too_long_for_the_width_scrolls_to_show_the_choice() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('t')));
    let all = rows_in(
        &render_at_width(&screen, 80),
        ratatui::layout::Rect::new(0, 0, 80, HEIGHT),
    )
    .concat();
    assert!(!all.contains("B:unison manual"), "{all}");
    assert!(all.contains("o:slide up/down"), "{all}");

    screen.handle_key_event(key(KeyCode::Char('B')));
    let buffer = render_at_width(&screen, 80);
    let rows = rows_in(&buffer, buffer.area);
    // B は o3 e に効かないので、matrix の見出しにも知らせとして出る。overlay の行は枠でない行。
    let pitch = rows
        .iter()
        .position(|row| !is_border_row(row) && row.contains("B:unison manual"))
        .unwrap_or_else(|| panic!("B:unison manual の行が無い: {rows:#?}"));
    assert!(!rows[pitch].contains("o:slide up/down"), "{}", rows[pitch]);
    assert!(is_highlighted(&buffer, "B:unison manual"));
    // 他の行はずらさない。
    assert!(
        rows[pitch - 1].contains("a:hammer/pull"),
        "{}",
        rows[pitch - 1]
    );
}

#[test]
fn the_choice_of_the_selected_lane_is_reversed_and_other_on_items_are_bracketed() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('t')));
    let buffer = render(&screen);
    assert!(is_highlighted(&buffer, NONE));
    assert!(!is_highlighted(&buffer, "a:hammer/pull"));

    // v で vibrato の段へ移る。vibrato は選ばれているので反転し、囲まない。
    screen.handle_key_event(key(KeyCode::Char('v')));
    let buffer = render(&screen);
    assert!(is_highlighted(&buffer, "v:vibrato"));
    assert!(!rows_in(&buffer, buffer.area)
        .concat()
        .contains("[v:vibrato]"));

    // KS の段へ戻って p を ON にすると、pinch harmonic が反転し、vibrato は [ ] で囲む。
    screen.handle_key_event(key(KeyCode::Char('k')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    let buffer = render(&screen);
    let all = rows_in(&buffer, buffer.area).concat();
    assert!(all.contains("[v:vibrato]"), "{all}");
    assert!(is_highlighted(&buffer, "p:pinch harmonic"));
    assert!(!all.contains("[p:pinch harmonic]"), "{all}");
    assert!(!is_highlighted(&buffer, NONE));
}
