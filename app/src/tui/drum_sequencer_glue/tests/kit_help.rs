use super::*;

fn help_shown(app: &TuiApp<'_>) -> bool {
    draw_selector(app).contains("ヘルプ(Keybinds)")
}

#[test]
fn question_mark_opens_help_over_selector_and_help_consumes_keys() {
    let mut app = app_with(ready());
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
    let before = selected(&app).selected().map(str::to_string);
    let _ = app.drum_sequencer.take_kit_audition();

    app.handle_drum_sequencer_key_event(press(KeyCode::Char('?')));
    assert!(help_shown(&app));
    for code in [
        KeyCode::Down,
        KeyCode::Char(' '),
        KeyCode::Enter,
        KeyCode::Char('/'),
        KeyCode::Char('q'),
    ] {
        assert_eq!(
            app.handle_drum_sequencer_key_event(press(code)),
            DrumSequencerAction::Continue
        );
    }
    assert_eq!(selected(&app).selected().map(str::to_string), before);
    assert!(!selected(&app).filter_editing());
    assert_eq!(app.drum_sequencer.take_kit_audition(), None);

    app.handle_drum_sequencer_key_event(press(KeyCode::Esc));
    assert!(!help_shown(&app));
    assert!(
        app.drum_sequencer.selector_open(),
        "Esc closes only the help"
    );

    app.handle_drum_sequencer_key_event(press(KeyCode::Char('?')));
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('q')));
    assert!(help_shown(&app), "q does not close the help");
    app.handle_drum_sequencer_key_event(press(KeyCode::Esc));
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('?')));
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('?')));
    assert!(!help_shown(&app));
}

#[test]
fn question_mark_is_typed_while_editing_the_regex() {
    let mut app = app_with(ready());
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('/')));
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('?')));
    assert!(!help_shown(&app));
    assert_eq!(
        cmrt_tui_core::text_input::textarea_value(selected(&app).query_textarea()),
        "?"
    );
}

#[test]
fn closing_selector_also_closes_help() {
    let mut app = app_with(ready());
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('?')));
    app.drum_sequencer.close_selector();
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
    assert!(!help_shown(&app));
}

/// 背面の「t で kit を選択」の全角文字が selector の左端へはみ出す幅で、枠の角が端末に届く。
#[test]
fn selector_corner_survives_wide_text_behind_its_left_edge() {
    let mut app = app_with(ready());
    app.switch_to_primary_screen(crate::screen_switch::PrimaryScreen::DrumSequencer, None);
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
    let lines = crate::tui::ui::tests::render_lines(&mut app, 120, 30);
    let row = lines
        .iter()
        .find(|line| line.contains("Regex"))
        .expect("selector query row");
    assert!(row.contains("┌ Regex"), "{row}");
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
fn help_key_column_splits_exactly_between_key_and_description() {
    use super::super::kit_help::{KEY_COLUMN_WIDTH, ROWS};
    assert_columns_aligned(&ROWS, KEY_COLUMN_WIDTH);
}
