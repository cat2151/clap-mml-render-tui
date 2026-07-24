use super::*;

#[test]
fn handle_normal_a_cycles_ab_repeat_and_tracks_cursor_until_end_is_fixed() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_measure = 1;

    app.handle_normal(crossterm::event::KeyCode::Char('a'));
    assert_eq!(
        app.ab_repeat_state(),
        AbRepeatState::FixStart {
            start_measure_index: 0,
            end_measure_index: 0,
        }
    );

    app.handle_normal(crossterm::event::KeyCode::Right);
    assert_eq!(
        app.ab_repeat_state(),
        AbRepeatState::FixStart {
            start_measure_index: 0,
            end_measure_index: 1,
        }
    );

    app.handle_normal(crossterm::event::KeyCode::Char('a'));
    assert_eq!(
        app.ab_repeat_state(),
        AbRepeatState::FixEnd {
            start_measure_index: 0,
            end_measure_index: 1,
        }
    );

    app.handle_normal(crossterm::event::KeyCode::Left);
    assert_eq!(
        app.ab_repeat_state(),
        AbRepeatState::FixEnd {
            start_measure_index: 0,
            end_measure_index: 1,
        }
    );

    app.handle_normal(crossterm::event::KeyCode::Char('a'));
    assert_eq!(app.ab_repeat_state(), AbRepeatState::Off);
}

#[test]
fn handle_normal_a_can_turn_off_ab_repeat_from_init_column() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_measure = 1;
    app.handle_normal(crossterm::event::KeyCode::Char('a'));
    app.handle_normal(crossterm::event::KeyCode::Char('a'));

    app.editor.cursor_measure = 0;
    app.handle_normal(crossterm::event::KeyCode::Char('a'));

    assert_eq!(app.ab_repeat_state(), AbRepeatState::Off);
}
