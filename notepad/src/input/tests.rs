use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tests::test_config;
use crate::{Mode, NormalAction, NotepadScreen, PlayState};

fn screen_on(lines: &[&str], cursor: usize) -> NotepadScreen<'static> {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.set_session_lines_for_test(lines.iter().map(|line| line.to_string()).collect());
    app.set_session_cursor_for_test(cursor);
    app
}

fn press(app: &mut NotepadScreen<'_>, code: KeyCode, modifiers: KeyModifiers) {
    assert!(matches!(
        app.handle_key_event(KeyEvent::new(code, modifiers)),
        NormalAction::Continue
    ));
}

#[test]
fn alt_arrows_reorder_the_current_line_and_keep_it_selected() {
    let mut app = screen_on(&["o4c", "o4d", "o4e"], 1);

    press(&mut app, KeyCode::Down, KeyModifiers::ALT);
    assert_eq!(app.session_lines(), ["o4c", "o4e", "o4d"]);
    assert_eq!(app.session_cursor(), 2);
    assert_eq!(app.editor.list_state.selected(), Some(2));

    press(&mut app, KeyCode::Up, KeyModifiers::ALT);
    press(&mut app, KeyCode::Up, KeyModifiers::ALT);
    assert_eq!(app.session_lines(), ["o4d", "o4c", "o4e"]);
    assert_eq!(app.session_cursor(), 0);
    assert_eq!(app.editor.list_state.selected(), Some(0));
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn alt_arrows_stop_at_the_first_and_last_line() {
    for (cursor, code) in [(0, KeyCode::Up), (2, KeyCode::Down)] {
        let mut app = screen_on(&["o4c", "o4d", "o4e"], cursor);

        press(&mut app, code, KeyModifiers::ALT);

        assert_eq!(app.session_lines(), ["o4c", "o4d", "o4e"]);
        assert_eq!(app.session_cursor(), cursor);
        assert_eq!(app.editor.list_state.selected(), Some(cursor));
        assert!(matches!(
            &*app.playback.session.play_state().lock().unwrap(),
            PlayState::Idle
        ));
    }

    for line in ["", "o4c"] {
        let mut app = screen_on(&[line], 0);
        for code in [KeyCode::Up, KeyCode::Down] {
            press(&mut app, code, KeyModifiers::ALT);
        }
        assert_eq!(app.session_lines(), [line]);
        assert_eq!(app.session_cursor(), 0);
        assert_eq!(app.editor.list_state.selected(), Some(0));
    }
}

#[test]
fn reordering_preserves_line_contents_playback_history_and_yank_buffer() {
    const MML: &str = r#"  {"Surge XT patch":"Pads/Pad 1.fxp"} o4c; o3g  "#;
    let mut app = screen_on(&[MML, "", MML], 0);
    app.editor.yank_buffer = Some("o4a".to_string());
    app.playback
        .session
        .set_play_state(PlayState::Playing("previous phrase".to_string()));
    let session = app
        .playback
        .session
        .session_token()
        .load(std::sync::atomic::Ordering::Acquire);
    let history = app.patch_phrase_store.notepad.history.clone();

    press(&mut app, KeyCode::Down, KeyModifiers::ALT);
    assert_eq!(app.session_lines(), ["", MML, MML]);
    press(&mut app, KeyCode::Down, KeyModifiers::ALT);
    assert_eq!(app.session_lines(), ["", MML, MML]);
    assert_eq!(app.session_cursor(), 2);
    assert_eq!(app.editor.list_state.selected(), Some(2));
    assert_eq!(app.editor.yank_buffer.as_deref(), Some("o4a"));
    assert_eq!(app.patch_phrase_store.notepad.history, history);
    assert_eq!(
        app.playback
            .session
            .session_token()
            .load(std::sync::atomic::Ordering::Acquire),
        session
    );
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Playing(message) if message == "previous phrase"
    ));
}

#[test]
fn alt_arrows_cancel_a_pending_double_d_delete_even_at_the_boundary() {
    for (cursor, code) in [(1, KeyCode::Down), (0, KeyCode::Up)] {
        let mut app = screen_on(&["o4c", "o4d", "o4e"], cursor);
        press(&mut app, KeyCode::Char('d'), KeyModifiers::NONE);
        assert!(app.editor.pending_delete);

        press(&mut app, code, KeyModifiers::ALT);
        assert!(!app.editor.pending_delete);
        press(&mut app, KeyCode::Char('d'), KeyModifiers::NONE);

        assert_eq!(app.session_lines().len(), 3);
        assert!(app.editor.pending_delete);
    }
}

#[test]
fn alt_shift_arrows_reorder_but_control_alt_arrows_do_nothing() {
    let mut app = screen_on(&["o4c", "o4d", "o4e"], 1);
    for code in [KeyCode::Up, KeyCode::Down] {
        press(&mut app, code, KeyModifiers::CONTROL | KeyModifiers::ALT);
        assert_eq!(app.session_lines(), ["o4c", "o4d", "o4e"]);
        assert_eq!(app.session_cursor(), 1);
        assert_eq!(app.editor.list_state.selected(), Some(1));
    }

    press(
        &mut app,
        KeyCode::Up,
        KeyModifiers::ALT | KeyModifiers::SHIFT,
    );
    assert_eq!(app.session_lines(), ["o4d", "o4c", "o4e"]);
    assert_eq!(app.session_cursor(), 0);
    assert_eq!(app.editor.list_state.selected(), Some(0));
}

#[test]
fn plain_arrows_still_navigate_and_play_without_reordering() {
    let mut app = screen_on(&["o4c", "o4d", "o4e"], 1);
    for (code, cursor, mml) in [(KeyCode::Down, 2, "o4e"), (KeyCode::Up, 1, "o4d")] {
        press(&mut app, code, KeyModifiers::NONE);

        assert_eq!(app.session_lines(), ["o4c", "o4d", "o4e"]);
        assert_eq!(app.session_cursor(), cursor);
        assert_eq!(app.editor.list_state.selected(), Some(cursor));
        assert!(matches!(
            &*app.playback.session.play_state().lock().unwrap(),
            PlayState::Running(message) if message == mml
        ));
    }
}

#[test]
fn alt_arrows_in_help_do_not_reorder_notepad_lines() {
    let mut app = screen_on(&["o4c", "o4d", "o4e"], 1);
    press(&mut app, KeyCode::Char('?'), KeyModifiers::NONE);
    for code in [KeyCode::Up, KeyCode::Down] {
        press(&mut app, code, KeyModifiers::ALT);
    }

    assert_eq!(app.mode, Mode::Help);
    assert_eq!(app.session_lines(), ["o4c", "o4d", "o4e"]);
    assert_eq!(app.session_cursor(), 1);
}
