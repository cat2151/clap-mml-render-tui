use crossterm::event::KeyCode;

use cmrt_tui_core::theme::{MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PURPLE};

use super::{key, label_fg, mark_cells, render, row_marks, rows_in, screen_with_mml};
use crate::ui::{layout_for, matrix, RuleLane, ROW_RULE_ROWS, RULE_LANES, RULE_ROWS};
use crate::Rule;

#[test]
fn every_rule_belongs_to_one_of_the_five_lanes() {
    for rule_row in RULE_ROWS {
        let lanes = RULE_LANES
            .iter()
            .filter(|lane| lane.rule_rows().any(|r| r.rule == rule_row.rule))
            .count();
        assert_eq!(lanes, 1, "{rule_row:?}");
    }
    let counts: Vec<usize> = RULE_LANES
        .iter()
        .map(|lane| lane.rule_rows().count())
        .collect();
    assert_eq!(counts, [27, 1, 1, 1, 2]);
    let release: Vec<Rule> = RuleLane::ReleaseShape.rule_rows().map(|r| r.rule).collect();
    assert_eq!(release, [Rule::PositionRelease, Rule::AutoSlideOut]);
}

#[test]
fn the_matrix_height_counts_five_column_rule_rows() {
    let screen = screen_with_mml("o3 l8 e f+ g");
    assert_eq!(
        usize::from(matrix::height(&screen)),
        1 + 3 + ROW_RULE_ROWS.len() + 5
    );
    assert_eq!(ROW_RULE_ROWS.len(), 4);
}

#[test]
fn ks_cells_show_the_overlay_letter_in_the_group_color() {
    let mut screen = screen_with_mml("o3 l8 e f+ g a");
    for ch in ['p', 'l', '/', 'l', 'g'] {
        screen.handle_key_event(key(KeyCode::Char(ch)));
    }
    let buffer = render(&screen);
    let matrix = layout_for(buffer.area, &screen).matrix;

    let cells = mark_cells(&buffer, matrix, "g:pick scratch");
    let marks: Vec<(&str, _)> = cells
        .iter()
        .map(|&cell| {
            let cell = buffer.cell(cell).unwrap();
            (cell.symbol(), cell.fg)
        })
        .collect();
    assert_eq!(
        marks,
        [
            ("p", MONOKAI_GREEN),
            ("o", MONOKAI_CYAN),
            ("g", MONOKAI_PURPLE),
        ]
    );
}

#[test]
fn the_ks_and_release_labels_name_the_rule_on_in_the_cursor_column() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('p')));
    let buffer = render(&screen);
    let matrix = layout_for(buffer.area, &screen).matrix;
    assert_eq!(label_fg(&buffer, matrix, "p:pinch harmonic"), MONOKAI_GREEN);
    assert_eq!(label_fg(&buffer, matrix, "t:release"), MONOKAI_GRAY);

    screen.handle_key_event(key(KeyCode::Char('l')));
    let buffer = render(&screen);
    assert_eq!(label_fg(&buffer, matrix, "t:KS"), MONOKAI_GRAY);
    assert_eq!(row_marks(&buffer, matrix, "t:KS"), "p");
    assert!(!rows_in(&buffer, matrix).concat().contains("pinch harmonic"));

    // 奏法リストの中の文字で切り替える。n = position rel、s = auto slide out。
    for (overlay_key, label, color) in [
        ('n', "t:position rel", MONOKAI_GREEN),
        ('s', "t:auto slide out", MONOKAI_CYAN),
    ] {
        for ch in ['t', overlay_key] {
            screen.handle_key_event(key(KeyCode::Char(ch)));
        }
        screen.handle_key_event(key(KeyCode::Esc));
        let buffer = render(&screen);
        assert_eq!(label_fg(&buffer, matrix, label), color, "{label}");
    }
    screen.handle_key_event(key(KeyCode::Char('h')));
    let buffer = render(&screen);
    assert_eq!(label_fg(&buffer, matrix, "t:release"), MONOKAI_GRAY);
    assert_eq!(row_marks(&buffer, matrix, "t:release"), "s");
}

#[test]
fn vibrato_and_a_ks_rule_in_one_column_show_on_two_rows() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('p')));
    screen.handle_key_event(key(KeyCode::Char('v')));
    let buffer = render(&screen);
    let matrix = layout_for(buffer.area, &screen).matrix;

    assert_eq!(row_marks(&buffer, matrix, "p:pinch harmonic"), "p");
    assert_eq!(row_marks(&buffer, matrix, "v:vibrato"), "v");
    let ks = mark_cells(&buffer, matrix, "p:pinch harmonic")[0];
    let vibrato = mark_cells(&buffer, matrix, "v:vibrato")[0];
    assert_eq!(ks.0, vibrato.0);
    assert_ne!(ks.1, vibrato.1);
}
