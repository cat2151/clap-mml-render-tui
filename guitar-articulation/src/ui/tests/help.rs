//! ヘルプ overlay は左 pane にキー、右 pane に詳しい解説を出す。

use super::*;

#[test]
fn question_mark_draws_the_help_over_the_screen() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    let buffer = render(&screen);
    let status = squeezed(&rows_in(&buffer, layout_for(buffer.area, &screen).status));
    assert!(status.contains("?:help"), "{status}");

    // ヘルプは 30 行の端末いっぱいに広がり、下段の 1 行も覆う。
    screen.handle_key_event(key(KeyCode::Char('?')));
    let buffer = render(&screen);

    let all = squeezed(&rows_in(&buffer, buffer.area));
    assert!(all.contains("ヘルプ(Keybinds)"), "{all}");
    let (keys, details) = crate::ui::help::pane_rects(buffer.area);
    let keys = squeezed(&rows_in(&buffer, keys));
    let details = squeezed(&rows_in(&buffer, details));
    assert!(keys.contains("weffectのdry/wet"), "{keys}");
    assert!(!keys.contains("Hammer-On"), "{keys}");
    assert!(details.contains("Hammer-On"), "{details}");
    assert!(details.contains("Mute_Down/Up"), "{details}");
}

#[test]
fn help_shows_every_row_of_both_panes_on_a_tall_terminal() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('?')));
    let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
    terminal.draw(|f| draw(&screen, f)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let (keys, details) = crate::ui::help::pane_rects(buffer.area);
    let keys = squeezed(&rows_in(&buffer, keys));
    let details = squeezed(&rows_in(&buffer, details));
    for row in crate::ui::help::KEY_ROWS {
        assert!(keys.contains(&row.replace(' ', "")), "{row}\n{keys}");
    }
    for row in crate::ui::help::DETAIL_ROWS {
        assert!(details.contains(&row.replace(' ', "")), "{row}\n{details}");
    }
}
