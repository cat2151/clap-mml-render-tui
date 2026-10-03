use cmrt_tui_core::theme::MONOKAI_YELLOW;
use crossterm::event::KeyCode;
use ratatui::{buffer::Buffer, layout::Rect};

use super::long_material::{long_mml_screen, long_smf_screen, render_playing};
use super::{key, render, rows_in};
use crate::ui::event_list::{name_width, EventRow};
use crate::ui::{layout_for, pane_block};
use crate::{GuitarArticulationScreen, Take, TimedMidiEvent};

/// 黄色の行の、pane の内側での上からの位置と中身。
fn yellow_rows(buffer: &Buffer, inner: Rect) -> Vec<(usize, String)> {
    rows_in(buffer, inner)
        .into_iter()
        .enumerate()
        .filter(|&(y, _)| buffer.cell((inner.x, inner.y + y as u16)).unwrap().fg == MONOKAI_YELLOW)
        .collect()
}

/// `take` のイベント列で、列 `column` のいちばん早い note on の行を描いた文字列（幅 `width` で切る）。
fn expected_text(
    screen: &GuitarArticulationScreen,
    take: Take,
    column: usize,
    width: u16,
) -> String {
    let events = screen.events(take);
    let on = screen.column_on_seconds(column, take).unwrap();
    let index = events
        .iter()
        .position(|e: &TimedMidiEvent| {
            e.message[0] & 0xF0 == 0x90 && e.message[2] != 0 && (e.seconds - on).abs() < 1e-9
        })
        .unwrap();
    let rows: Vec<EventRow> = events
        .iter()
        .map(|event| EventRow::new(event, take == Take::Converted))
        .collect();
    rows[index]
        .text(name_width(&rows))
        .chars()
        .take(usize::from(width))
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// 左右の pane それぞれで、黄色の行がただ 1 つ、上から `lead(高さ)` 行目に在り、列 `column` の note on であること。
fn assert_target(
    buffer: &Buffer,
    screen: &GuitarArticulationScreen,
    column: usize,
    lead: fn(usize) -> usize,
) {
    let layout = layout_for(buffer.area, screen);
    for (take, area) in [
        (Take::Plain, layout.plain),
        (Take::Converted, layout.converted),
    ] {
        let inner = pane_block("").inner(area);
        let yellow = yellow_rows(buffer, inner);
        assert_eq!(yellow.len(), 1, "{take:?}: {yellow:#?}");
        let (y, text) = &yellow[0];
        assert_eq!(*y, lead(usize::from(inner.height)), "{take:?}");
        assert_eq!(
            text.trim_end(),
            expected_text(screen, take, column, inner.width),
            "{take:?}"
        );
    }
}

fn middle(height: usize) -> usize {
    height / 2
}

fn third(height: usize) -> usize {
    height / 3
}

fn move_cursor_to(screen: &mut GuitarArticulationScreen, column: usize) {
    for _ in 0..column {
        screen.handle_key_event(key(KeyCode::Char('l')));
    }
    assert_eq!(screen.cursor(), column);
}

#[test]
fn while_playing_a_long_smf_both_lists_center_the_note_on_being_played() {
    let mut screen = long_smf_screen(300);
    move_cursor_to(&mut screen, 100);
    let buffer = render_playing(&mut screen, Take::Converted, 150);

    assert_target(&buffer, &screen, 150, middle);
}

#[test]
fn when_playing_stops_the_lists_follow_the_cursor_a_third_down_again() {
    let mut screen = long_smf_screen(300);
    move_cursor_to(&mut screen, 100);
    render_playing(&mut screen, Take::Converted, 150);
    screen.set_playhead(None);
    let buffer = render(&screen);

    assert_target(&buffer, &screen, 100, third);
}

#[test]
fn an_mml_material_also_centers_the_note_on_being_played() {
    let mut screen = long_mml_screen();
    let buffer = render_playing(&mut screen, Take::Plain, 70);

    assert_target(&buffer, &screen, 70, middle);
}

#[test]
fn before_the_first_note_the_lists_follow_the_cursor() {
    let mut screen = long_smf_screen(300);
    move_cursor_to(&mut screen, 100);
    let first = screen.column_on_seconds(0, Take::Converted).unwrap();
    screen.set_playhead(Some((Take::Converted, first - 0.001)));
    assert_eq!(screen.playhead_column(), None);
    let buffer = render(&screen);

    assert_target(&buffer, &screen, 100, third);
}
