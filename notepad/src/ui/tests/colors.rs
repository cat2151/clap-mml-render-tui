use super::*;

#[test]
fn normal_screen_uses_monokai_background_and_border_color() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["abc".to_string()];

    let buffer = render_buffer(&mut app, 80, 8);

    assert_eq!(buffer.cell((0, 0)).unwrap().fg, MONOKAI_CYAN);
    assert_eq!(buffer.cell((0, 0)).unwrap().bg, MONOKAI_BG);
    assert_eq!(buffer.cell((4, 4)).unwrap().fg, MONOKAI_CYAN);
    assert_eq!(buffer.cell((4, 4)).unwrap().bg, MONOKAI_BG);
}

#[test]
fn help_screen_uses_light_gray_escape_hint_on_monokai_background() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.mode = Mode::Help;

    let buffer = render_buffer(&mut app, 80, 60);
    let (x, y) = find_text(&buffer, "[ESC]");

    assert_eq!(buffer.cell((x, y)).unwrap().fg, MONOKAI_GRAY);
    assert_eq!(buffer.cell((x, y)).unwrap().bg, MONOKAI_BG);
}

#[test]
fn status_color_uses_monokai_palette() {
    assert_eq!(status_color(&PlayState::Idle), MONOKAI_CYAN);
    assert_eq!(
        status_color(&PlayState::Running("render".to_string())),
        MONOKAI_PURPLE
    );
    assert_eq!(
        status_color(&PlayState::Playing("play".to_string())),
        MONOKAI_YELLOW
    );
    assert_eq!(
        status_color(&PlayState::Done("done".to_string())),
        MONOKAI_GREEN
    );
    assert_eq!(status_color(&PlayState::Err("err".to_string())), Color::Red);
}

#[test]
fn patch_phrase_screen_uses_monokai_foreground_for_unfocused_list() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.mode = Mode::PatchPhrase;
    app.patch_phrase.patch_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );

    let buffer = render_buffer(&mut app, 80, 10);
    let (x, y) = find_text(&buffer, "o5g");

    assert_eq!(buffer.cell((x, y)).unwrap().fg, MONOKAI_FG);
    assert_eq!(buffer.cell((x, y)).unwrap().bg, MONOKAI_BG);
}

#[test]
fn normal_screen_highlights_patch_stem_and_note_names() {
    assert_highlighted_row(0);
}

#[test]
fn normal_screen_highlights_the_cursor_row_too() {
    assert_highlighted_row(1);
}

fn assert_highlighted_row(cursor: usize) {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![
        "t".to_string(),
        r#"{"Surge XT patch": "Pads/Warm.fxp"} t120 c"#.to_string(),
    ];
    app.editor.cursor = cursor;
    app.editor.list_state.select(Some(cursor));

    let buffer = render_buffer(&mut app, 80, 8);
    let y = find_text(&buffer, "Warm").1;
    let row: Vec<String> = (0..80)
        .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
        .collect();
    let cell_at = |text: &str| {
        let x = (0..row.len())
            .find(|&x| row[x..].concat().starts_with(text))
            .unwrap();
        buffer.cell((x as u16, y)).unwrap().clone()
    };
    let fg_at = |text: &str| cell_at(text).fg;

    assert_eq!(fg_at("Pads/"), MONOKAI_YELLOW);
    assert_eq!(fg_at("Warm"), MONOKAI_GREEN);
    assert_eq!(fg_at(".fxp"), MONOKAI_YELLOW);
    assert_eq!(fg_at("t120"), MONOKAI_FG);
    assert_eq!(fg_at("c "), MONOKAI_PINK);
    assert_eq!(cell_at("Warm").bg != MONOKAI_BG, cursor == 1);
}
