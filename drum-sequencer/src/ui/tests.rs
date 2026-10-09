use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

use super::*;
use crate::{DrumHit, DrumPattern, DEFAULT_VELOCITY};

fn hit(note: u8, step: usize, steps: u8) -> DrumHit {
    DrumHit {
        note,
        step,
        steps,
        velocity: DEFAULT_VELOCITY,
    }
}

fn press(screen: &mut DrumSequencerScreen, code: KeyCode, count: usize) {
    for _ in 0..count {
        screen.handle_key_event(KeyEvent::new(code, KeyModifiers::NONE));
    }
}

fn render(screen: &mut DrumSequencerScreen, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| draw(screen, frame)).unwrap();
    terminal.backend().buffer().clone()
}

fn rows(buffer: &Buffer, area: Rect) -> Vec<String> {
    (area.y..area.bottom())
        .map(|y| {
            (area.x..area.right())
                .map(|x| buffer.cell((x, y)).unwrap().symbol())
                .collect()
        })
        .collect()
}

#[test]
fn vertical_scroll_keeps_the_cursor_visible_up_down_and_after_resize() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit(
        "full range".into(),
        Some((0..128).collect()),
        Vec::new(),
        Vec::new(),
    );
    press(&mut screen, KeyCode::Char('k'), 100);
    for (down, height, note) in [(0, 12, 100), (8, 12, 92), (0, 30, 92)] {
        press(&mut screen, KeyCode::Down, down);
        let buffer = render(&mut screen, 80, height);
        let matrix = rows(&buffer, layout_for(buffer.area).matrix);
        let selected: Vec<_> = matrix.iter().filter(|row| row.contains("[.]")).collect();
        assert_eq!(selected.len(), 1, "{matrix:?}");
        assert!(selected[0].starts_with(&format!("{note:>3} ")));
        // 見えている行は、上から下へ note が 1 ずつ下がる。
        let notes: Vec<u8> = matrix
            .iter()
            .map(|row| row[..3].trim().parse().unwrap())
            .collect();
        assert!(
            notes.windows(2).all(|pair| pair[0] == pair[1] + 1),
            "{notes:?}"
        );
    }
}

#[test]
fn first_kit_shows_its_lowest_note_at_the_bottom_with_the_cursor() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit(
        "full range".into(),
        Some((0..128).collect()),
        Vec::new(),
        Vec::new(),
    );
    let buffer = render(&mut screen, 80, 12);
    let matrix = rows(&buffer, layout_for(buffer.area).matrix);
    let last = matrix.last().unwrap();
    assert!(last.starts_with("  0 "), "{matrix:?}");
    assert!(last.contains("[.]"));
}

#[test]
fn higher_notes_are_drawn_above_with_their_names_clipped_to_the_label_column() {
    let long = "Very Long Crash Cymbal Name That Does Not Fit";
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit(
        "kit".into(),
        Some(vec![36, 49, 38]),
        vec![(36, "Kick".into()), (49, long.into())],
        Vec::new(),
    );
    press(&mut screen, KeyCode::Up, 1);
    press(&mut screen, KeyCode::Char(' '), 1);
    assert!(screen.cell_on(38, 0));
    let buffer = render(&mut screen, 80, 14);
    let matrix = rows(&buffer, layout_for(buffer.area).matrix);
    let label = usize::from(label_width(layout_for(buffer.area).matrix.width));
    let row_of = |note: u8| {
        matrix
            .iter()
            .position(|row| row.starts_with(&format!("{note:>3} ")))
            .unwrap()
    };
    assert!(
        row_of(49) < row_of(38) && row_of(38) < row_of(36),
        "{matrix:?}"
    );
    assert!(matrix[row_of(36)].starts_with(" 36 Kick "));
    assert!(matrix[row_of(38)].starts_with(&format!(" 38 {UNKNOWN_NAME} ")));
    let crash = &matrix[row_of(49)];
    assert!(crash.starts_with(" 49 Very Long"));
    assert!(!crash.contains(long), "long name must be clipped: {crash}");
    // 左列の右端は空白で、step 列は 16 個そのまま残る。
    for row in &matrix[..3] {
        assert_eq!(&row[label - 1..label], " ", "{row}");
        let steps = &row[label..label + DRUM_STEPS * STEP_WIDTH];
        assert_eq!(
            steps
                .chars()
                .filter(|ch| matches!(ch, '.' | 'x' | '-'))
                .count(),
            16
        );
    }
    // one-shot でない 38 は 4 分音符鳴るので、続く 3 step に `-` が続く。
    assert!(matrix[row_of(38)][label..].starts_with("[x] -  -  - "));
    assert_eq!(matrix[row_of(38)][label..].matches(" - ").count(), 3);
    let cursor = rows(&buffer, layout_for(buffer.area).cursor).concat();
    assert!(cursor.contains("Note 38 ?"), "{cursor}");

    // 狭い端末では名前を落とし、note number と 16 step を残す。
    let buffer = render(&mut screen, 54, 14);
    let matrix = rows(&buffer, layout_for(buffer.area).matrix);
    assert_eq!(
        usize::from(label_width(layout_for(buffer.area).matrix.width)),
        4
    );
    assert!(matrix[0].starts_with(" 49 "));
    assert_eq!(matrix[0][4..].chars().filter(|ch| *ch == '.').count(), 16);
}

#[test]
fn help_overlay_is_drawn_over_the_matrix_and_closing_keeps_the_matrix() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".into(), Some(vec![36]), Vec::new(), Vec::new());
    let text = |buffer: &Buffer| rows(buffer, buffer.area).concat();
    assert!(!text(&render(&mut screen, 100, 30)).contains("Keybinds"));
    press(&mut screen, KeyCode::Char('?'), 1);
    let shown = text(&render(&mut screen, 100, 30));
    assert!(shown.contains("Keybinds"), "{shown}");
    assert!(shown.contains("Shift+P"));
    // 80x24 でも左 pane の最終行と右 pane の最終行が切れずに見える。
    // 全角の後ろの継続セルは空白になるので、空白を除いて探す。
    let small: String = text(&render(&mut screen, 80, 24))
        .chars()
        .filter(|ch| *ch != ' ')
        .collect();
    assert!(small.contains("helpを閉じる"), "{small}");
    assert!(small.contains("xは黄色"), "{small}");
    press(&mut screen, KeyCode::Esc, 1);
    let buffer = render(&mut screen, 100, 30);
    assert!(!text(&buffer).contains("Keybinds"));
    assert!(rows(&buffer, layout_for(buffer.area).matrix)[0].contains("[.]"));
}

#[test]
fn matrix_renders_all_sixteen_steps_and_retains_them_in_narrow_terminals() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".into(), Some(vec![36]), Vec::new(), Vec::new());
    press(&mut screen, KeyCode::Char('l'), 15);
    press(&mut screen, KeyCode::Enter, 1);
    let buffer = render(&mut screen, 80, 12);
    let layout = layout_for(buffer.area);
    let headings = rows(&buffer, layout.steps);
    for step in 1..=DRUM_STEPS {
        assert!(headings[0].contains(&format!("{step:^STEP_WIDTH$}")));
    }
    let matrix = rows(&buffer, layout.matrix);
    let label = usize::from(label_width(layout.matrix.width));
    assert!(matrix[0].starts_with(" 36 "));
    assert_eq!(matrix[0].chars().filter(|ch| *ch == '.').count(), 15);
    assert_eq!(matrix[0].find("[x]"), Some(label + 15 * STEP_WIDTH));
    // 見出しの step 1 は matrix の step 1 と同じ桁にある。
    assert_eq!(&headings[0][label..label + STEP_WIDTH], " 1 ");

    for (width, height) in [(0, 0), (1, 1), (8, 3), (20, 10), (53, 12)] {
        render(&mut screen, width, height);
        assert_eq!(screen.notes(), [36]);
        assert_eq!(screen.cursor_step(), 15);
        assert!(screen.cell_on(36, 15));
    }
    let buffer = render(&mut screen, 80, 12);
    assert!(rows(&buffer, layout_for(buffer.area).matrix)[0].contains("[x]"));
}

#[test]
fn empty_views_explain_unselected_missing_catalog_and_known_empty_notes() {
    let mut screen = DrumSequencerScreen::default();
    let buffer = render(&mut screen, 80, 14);
    let text = rows(&buffer, layout_for(buffer.area).matrix).concat();
    assert!(text.contains("t"));
    assert!(!text.contains("[.]"));

    screen.set_kit("old catalog".into(), None, Vec::new(), Vec::new());
    let buffer = render(&mut screen, 80, 14);
    let text: String = rows(&buffer, layout_for(buffer.area).matrix)
        .concat()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    assert!(text.contains("cmrtbuild-patch-catalog-cache"));

    screen.set_kit("empty".into(), Some(vec![]), Vec::new(), Vec::new());
    let buffer = render(&mut screen, 80, 14);
    let text = rows(&buffer, layout_for(buffer.area).matrix).concat();
    assert!(!text.contains("build-patch-catalog-cache"));
    assert!(!text.contains("[.]"));
}

#[test]
fn restored_kits_say_whether_they_wait_for_or_are_missing_from_the_catalog() {
    let mut screen = DrumSequencerScreen::default();
    screen.restore(
        Some("Saved Kit".into()),
        [(0, DrumPattern::from_hits([hit(36, 0, 1)]))],
        0,
        Some(36),
        0,
    );
    let waiting = render(&mut screen, 80, 14);
    let layout = layout_for(waiting.area);
    // 全角文字の後ろの空きセルを除いて比べる。
    let text = |buffer: &Buffer, area: Rect| -> String {
        rows(buffer, area)
            .concat()
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect()
    };
    let kit_line = text(&waiting, layout.kit);
    assert!(kit_line.contains("SavedKit") && kit_line.contains("照合中"));
    let waiting_text = text(&waiting, layout.matrix);

    screen.mark_kit_missing();
    let missing = render(&mut screen, 80, 14);
    assert!(text(&missing, layout.kit).contains("catalogに無い"));
    let missing_text = text(&missing, layout.matrix);
    assert_ne!(waiting_text, missing_text);
    assert!(!missing_text.contains("build-patch-catalog-cache"));
    assert!(!missing_text.contains("[.]"));
}

#[test]
fn the_playhead_marks_only_its_step_number_and_the_hits_being_played() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".into(), Some(vec![36, 38]), Vec::new(), Vec::new());
    press(&mut screen, KeyCode::Char(' '), 1);
    press(&mut screen, KeyCode::Char('l'), 3);
    press(&mut screen, KeyCode::Char(' '), 1);
    press(&mut screen, KeyCode::Char('l'), 5);
    let marked = |buffer: &Buffer| -> Vec<u16> {
        let steps = layout_for(buffer.area).steps;
        (steps.x..steps.right())
            .filter(|&x| {
                buffer
                    .cell((x, steps.y))
                    .unwrap()
                    .modifier
                    .contains(Modifier::REVERSED)
            })
            .collect()
    };
    let hit_color = |buffer: &Buffer, step: usize| {
        let layout = layout_for(buffer.area);
        let label = label_width(layout.matrix.width);
        let x = layout.matrix.x + label + (step * STEP_WIDTH) as u16 + 1;
        // 2 行（38, 36）の下の行が 36。
        let y = layout.matrix.y + 1;
        assert_eq!(buffer.cell((x, y)).unwrap().symbol(), "x");
        buffer.cell((x, y)).unwrap().fg
    };

    let stopped = render(&mut screen, 80, 12);
    assert!(marked(&stopped).is_empty());
    assert_eq!(hit_color(&stopped, 3), MONOKAI_GREEN);

    screen.set_playhead(Some(3));
    let playing = render(&mut screen, 80, 12);
    let steps = layout_for(playing.area).steps;
    let number: String = marked(&playing)
        .iter()
        .map(|&x| playing.cell((x, steps.y)).unwrap().symbol())
        .collect();
    assert_eq!(number.trim(), "4", "only step 4 (index 3) is marked");
    assert_eq!(hit_color(&playing, 3), MONOKAI_YELLOW);
    assert_eq!(hit_color(&playing, 0), MONOKAI_GREEN);

    screen.set_playhead(Some(DRUM_STEPS));
    assert_eq!(screen.playhead(), None, "out of range is not shown");
    screen.set_playhead(None);
    assert!(marked(&render(&mut screen, 80, 12)).is_empty());
}

#[test]
fn every_other_beat_of_the_grid_is_shaded() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".into(), Some(vec![36]), Vec::new(), Vec::new());
    // カーソルは step 1（塗らない拍）にあり、帯の色だけを見られる。
    let buffer = render(&mut screen, 80, 12);
    let layout = layout_for(buffer.area);
    let label = label_width(layout.matrix.width);
    let bg = |y: u16, step: usize| {
        let x = layout.matrix.x + label + (step * STEP_WIDTH) as u16;
        buffer.cell((x, y)).unwrap().bg
    };
    for y in [layout.steps.y, layout.matrix.y] {
        let shaded: Vec<usize> = (0..DRUM_STEPS)
            .filter(|&step| bg(y, step) == BEAT_BAND_BG)
            .collect();
        assert_eq!(shaded, [4, 5, 6, 7, 12, 13, 14, 15], "row {y}");
    }
}

#[test]
fn lengths_trail_their_hit_until_the_next_hit_of_the_note_and_the_header_names_the_pattern() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".into(), Some(vec![36, 46]), Vec::new(), vec![36]);
    screen.set_patterns([(
        2,
        DrumPattern::from_hits([hit(46, 0, 16), hit(46, 4, 2), hit(46, 14, 8), hit(36, 1, 3)]),
    )]);
    press(&mut screen, KeyCode::Char(']'), 2);
    let buffer = render(&mut screen, 80, 14);
    let layout = layout_for(buffer.area);
    let label = usize::from(label_width(layout.matrix.width));
    let matrix = rows(&buffer, layout.matrix);
    let cells = |row: &str| -> String {
        row[label..label + DRUM_STEPS * STEP_WIDTH]
            .chars()
            .filter(|ch| !matches!(ch, ' ' | '[' | ']'))
            .collect()
    };
    assert_eq!(cells(&matrix[0]), "x---x-........x-");
    // one-shot の 36 も、編集した長さだけ `-` を描く。
    assert_eq!(cells(&matrix[1]), ".x--............");
    assert!(rows(&buffer, layout.kit)
        .concat()
        .contains("Pattern 2  Kit: kit"));
    let cursor = rows(&buffer, layout.cursor).concat();
    assert!(cursor.contains("Note 36 ? (one-shot)"), "{cursor}");
    press(&mut screen, KeyCode::Char('l'), 1);
    let buffer = render(&mut screen, 80, 14);
    let cursor = rows(&buffer, layout.cursor).concat();
    // 全角文字の後ろには空きセルが入るので、数字の部分だけを見る。
    assert!(
        cursor.contains("ON") && cursor.contains(" 3/16 step"),
        "{cursor}"
    );
}

#[test]
fn pending_count_is_shown_at_the_right_end_of_the_status_row_until_consumed() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".into(), Some(vec![36]), Vec::new(), Vec::new());
    let status_row = |screen: &mut DrumSequencerScreen| {
        let buffer = render(screen, 80, 12);
        rows(&buffer, layout_for(buffer.area).status).concat()
    };
    assert_eq!(status_row(&mut screen).trim(), "");
    press(&mut screen, KeyCode::Char('1'), 1);
    press(&mut screen, KeyCode::Char('6'), 1);
    let row = status_row(&mut screen);
    assert!(row.ends_with(" 16"), "{row:?}");
    press(&mut screen, KeyCode::Char('l'), 1);
    assert_eq!(status_row(&mut screen).trim(), "");
}

/// 列幅の定数がずれると、キーの途中や説明の途中で色が変わる。
fn assert_columns_aligned(rows: &[&'static str], key_width: usize) {
    for row in rows {
        let line = cmrt_tui_core::help_line::help_line(row, key_width);
        if let [key, description] = &line.spans[..] {
            assert!(key.content.ends_with(' '), "{row:?}");
            let blank_key = key.content.trim().is_empty();
            assert!(
                blank_key || !description.content.starts_with(' '),
                "{row:?}"
            );
        }
    }
}

#[test]
fn help_key_columns_split_exactly_between_key_and_description() {
    assert_columns_aligned(&help::KEY_ROWS, help::KEY_COLUMN_WIDTH);
    assert_columns_aligned(&help::DETAIL_ROWS, help::DETAIL_COLUMN_WIDTH);
}
