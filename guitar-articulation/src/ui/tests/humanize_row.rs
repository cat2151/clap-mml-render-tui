use cmrt_tui_core::theme::MONOKAI_YELLOW;
use crossterm::event::KeyCode;

use super::{key, render, row_marks, rows_in, screen_with_mml, squeezed};
use crate::ui::layout_for;
use crate::Take;

const MML: &str = "o3 l16 e f+ g a b a g f+";

#[test]
fn the_humanize_row_shows_the_shift_of_each_column_only_while_on() {
    let mut screen = screen_with_mml(MML);
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);
    let rows = rows_in(&buffer, layout.matrix);
    let humanize = rows.iter().position(|r| r.contains("d:humanize")).unwrap();
    let eco = rows
        .iter()
        .position(|r| r.contains("e:economy picking"))
        .unwrap();
    // 間に汚し（リリース）の段が 1 つ入る。
    assert_eq!(humanize + 2, eco, "{rows:#?}");
    assert_eq!(
        row_marks(&buffer, layout.matrix, "d:humanize"),
        "",
        "{rows:#?}"
    );

    screen.handle_key_event(key(KeyCode::Char('d')));
    let buffer = render(&screen);
    let marks = row_marks(
        &buffer,
        layout_for(buffer.area, &screen).matrix,
        "d:humanize",
    );
    assert_eq!(marks.chars().count(), screen.column_count(), "{marks}");
    assert!(marks.chars().all(|ch| "<·>".contains(ch)), "{marks}");
    // 8 列の揺れが全部同じ向きにはならない。
    assert!(
        marks.chars().any(|ch| ch != marks.chars().next().unwrap()),
        "{marks}"
    );
}

#[test]
fn the_humanize_key_is_in_the_keybinds_and_the_help() {
    let mut screen = screen_with_mml(MML);
    let buffer = render(&screen);
    let status = squeezed(&rows_in(&buffer, layout_for(buffer.area, &screen).status));
    assert!(status.contains("d:汚し"), "{status}");
    assert!(status.contains("?:help"), "{status}");

    screen.handle_key_event(key(KeyCode::Char('?')));
    let all = squeezed(&rows_in(&render(&screen), render(&screen).area));
    assert!(all.contains("汚し(humanize行)"), "{all}");
}

/// Articulated の pane で黄色く強調された行。
fn highlighted_converted_row(screen: &crate::GuitarArticulationScreen) -> String {
    let buffer = render(screen);
    let area = layout_for(buffer.area, screen).converted;
    let rows = rows_in(&buffer, area);
    let y = (area.y..area.y + area.height)
        .find(|&y| buffer.cell((area.x + 3, y)).unwrap().fg == MONOKAI_YELLOW)
        .unwrap_or_else(|| panic!("強調された行が無い: {rows:#?}"));
    rows[usize::from(y - area.y)].clone()
}

#[test]
fn the_cursor_column_note_on_is_highlighted_even_when_humanized() {
    let mut screen = screen_with_mml(MML);
    for _ in 0..3 {
        screen.handle_key_event(key(KeyCode::Char('l')));
    }
    let written = screen.column_on_seconds(3, Take::Plain).unwrap();
    let just = highlighted_converted_row(&screen);
    assert!(just.contains(&format!("{written:.3} on")), "{just}");

    screen.handle_key_event(key(KeyCode::Char('d')));
    let played = screen.column_on_seconds(3, Take::Converted).unwrap();
    assert_ne!(format!("{played:.3}"), format!("{written:.3}"));
    let row = highlighted_converted_row(&screen);
    assert!(row.contains(&format!("{played:.3} on")), "{row}");
}

#[test]
fn the_release_row_sits_under_the_humanize_row_and_marks_every_column_only_while_on() {
    let mut screen = screen_with_mml(MML);
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);
    let rows = rows_in(&buffer, layout.matrix);
    let humanize = rows.iter().position(|r| r.contains("d:humanize")).unwrap();
    let release = rows
        .iter()
        .position(|r| r.contains("r:humanize release"))
        .unwrap();
    assert_eq!(humanize + 1, release, "{rows:#?}");
    assert_eq!(
        row_marks(&buffer, layout.matrix, "r:humanize release"),
        "",
        "{rows:#?}"
    );

    screen.handle_key_event(key(KeyCode::Char('r')));
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);
    let marks = row_marks(&buffer, layout.matrix, "r:humanize release");
    assert_eq!(marks, "~".repeat(screen.column_count()), "{marks}");
    // d の段は r の ON で変わらない。
    assert_eq!(row_marks(&buffer, layout.matrix, "d:humanize"), "");
}

#[test]
fn the_release_key_is_in_the_keybinds_and_the_help() {
    let mut screen = screen_with_mml(MML);
    let buffer = render(&screen);
    let status = squeezed(&rows_in(&buffer, layout_for(buffer.area, &screen).status));
    assert!(status.contains("r:rel"), "{status}");
    assert!(status.contains("?:help"), "{status}");
    // status から外した `i` は MML の pane の見出しに在る。
    let input = squeezed(&rows_in(&buffer, layout_for(buffer.area, &screen).input));
    assert!(input.contains("MML(i)"), "{input}");

    screen.handle_key_event(key(KeyCode::Char('?')));
    let all = squeezed(&rows_in(&render(&screen), render(&screen).area));
    assert!(all.contains("汚し(リリース音)"), "{all}");
}

#[test]
fn the_cursor_column_note_on_is_highlighted_with_release_on_and_humanize_on_or_off() {
    let mut screen = screen_with_mml(MML);
    for _ in 0..3 {
        screen.handle_key_event(key(KeyCode::Char('l')));
    }
    screen.handle_key_event(key(KeyCode::Char('r')));
    let written = screen.column_on_seconds(3, Take::Converted).unwrap();
    let row = highlighted_converted_row(&screen);
    assert!(row.contains(&format!("{written:.3} on")), "{row}");

    screen.handle_key_event(key(KeyCode::Char('d')));
    let played = screen.column_on_seconds(3, Take::Converted).unwrap();
    assert_ne!(format!("{played:.3}"), format!("{written:.3}"));
    let row = highlighted_converted_row(&screen);
    assert!(row.contains(&format!("{played:.3} on")), "{row}");
}
