use cmrt_tui_core::theme::{MONOKAI_DARK_GRAY, MONOKAI_PINK};
use ratatui::{buffer::Buffer, layout::Rect, style::Color};

use crossterm::event::KeyCode;

use super::{key, render, rows_in, screen_with_mml};
use crate::ui::event_list::EventRow;
use crate::ui::layout_for;
use crate::TimedMidiEvent;

/// 行の中の `text` の頭の文字の色。
fn fg_of(buffer: &Buffer, area: Rect, row: &str, text: &str) -> Color {
    let rows = rows_in(buffer, area);
    let y = rows.iter().position(|r| r == row).unwrap();
    let x = row[..row.rfind(text).unwrap()].chars().count();
    buffer
        .cell((area.x + x as u16, area.y + y as u16))
        .unwrap()
        .fg
}

#[test]
fn loud_velocities_are_pink_and_note_offs_are_dark_gray() {
    // 頭の音はエコノミーピッキングでも元の velocity（127）のまま。
    let screen = screen_with_mml("o3 l8 e f+ g");
    let buffer = render(&screen);
    let area = layout_for(buffer.area, &screen).converted;
    let rows = rows_in(&buffer, area);

    let loud = rows.iter().find(|r| r.contains(" on  E2 ")).unwrap();
    assert_eq!(fg_of(&buffer, area, loud, "127"), MONOKAI_PINK, "{rows:#?}");
    let keyswitch = rows.iter().find(|r| r.contains("KS Sus_Down")).unwrap();
    assert_ne!(
        fg_of(&buffer, area, keyswitch, "127"),
        MONOKAI_PINK,
        "KS の velocity は演奏の強さではない: {rows:#?}"
    );
    let off = rows.iter().find(|r| r.contains(" off E2 ")).unwrap();
    assert_eq!(
        fg_of(&buffer, area, off, "off"),
        MONOKAI_DARK_GRAY,
        "{rows:#?}"
    );
    assert_eq!(
        fg_of(&buffer, area, off, "E2"),
        MONOKAI_DARK_GRAY,
        "{rows:#?}"
    );
}

fn row_text(message: [u8; 3], name_keyswitches: bool) -> String {
    let event = TimedMidiEvent {
        seconds: 0.5,
        message,
    };
    EventRow::new(&event, name_keyswitches).text(13)
}

#[test]
fn keyswitches_outside_the_articulations_take_their_ks_map_names() {
    assert_eq!(
        row_text([0x90, 13, 127], true),
        "  0.500 on  KS Brush_Alt      127"
    );
    assert_eq!(
        row_text([0x80, 27, 0], true),
        "  0.500 off KS Slide_In       off"
    );
    // raw は今までどおり音名。
    assert_eq!(
        row_text([0x90, 13, 127], false),
        "  0.500 on  C#0               127"
    );
}

#[test]
fn the_manual_controls_have_names() {
    assert_eq!(
        row_text([0xB0, 21, 64], true),
        "  0.500 cc  vibrato speed      64"
    );
    assert_eq!(
        row_text([0xB0, 22, 10], true),
        "  0.500 cc  mute length        10"
    );
    assert_eq!(
        row_text([0xB0, 27, 3], true),
        "  0.500 cc  slide-in range       3"
    );
    assert_eq!(
        row_text([0xB0, 26, 104], true),
        "  0.500 cc  slide up/down range     104"
    );
}

#[test]
fn pitch_bends_show_the_signed_offset_from_the_center() {
    // 9000 - 8192 = +808、0 - 8192 = -8192。
    assert_eq!(
        row_text([0xE0, 40, 70], true),
        "  0.500 pb                    808"
    );
    assert_eq!(
        row_text([0xE0, 0, 0], true),
        "  0.500 pb                    -8192"
    );
    assert_eq!(
        row_text([0xE0, 0, 0x40], true),
        "  0.500 pb                      0"
    );
}

#[test]
fn the_note_preview_lists_only_the_cursor_column_on_the_right() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('n')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    let right = rows_in(&buffer, layout.converted).join(
        "
",
    );
    assert!(right.contains(" on  F#2 "), "{right}");
    assert!(!right.contains("E2 "), "{right}");
    assert!(!right.contains("G2 "), "{right}");
    let left = rows_in(&buffer, layout.plain).join(
        "
",
    );
    for pitch in [" on  E2 ", " on  F#2 ", " on  G2 "] {
        assert!(left.contains(pitch), "左 pane は全体のまま: {left}");
    }
}
