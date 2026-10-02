use super::*;

const COMMAND: &str = "cmrt kb -p \"patches_3rdparty/John Valentine/Plucks/Guitarp.fxp\" -t auto -n 60,64,67 -e \"Surge XT Effects preset=Delay/Rhythmic 1.srgfx\"";

#[test]
fn the_share_notice_shows_the_message_and_the_whole_command() {
    let mut terminal = Terminal::new(TestBackend::new(74, 12)).unwrap();
    terminal
        .draw(|f| crate::ui::share_notice::draw_share_notice_overlay(Some(COMMAND), f, f.area()))
        .unwrap();
    let rendered = buffer_to_string(&terminal);

    // 全角文字の右半分のセルは空白なので、空白を抜いて探す。
    assert!(rendered
        .replace(' ', "")
        .contains("クリップボードにコピーしました"));
    // 折り返した行を枠の内側だけつなぎ直すと、コマンド全文に戻る。
    let joined: String = rendered
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix('│')?
                .strip_suffix('│')
                .map(str::trim_end)
        })
        .skip(1)
        .collect();
    assert_eq!(joined, COMMAND);
}

#[test]
fn no_notice_draws_nothing() {
    let mut terminal = Terminal::new(TestBackend::new(74, 12)).unwrap();
    terminal
        .draw(|f| crate::ui::share_notice::draw_share_notice_overlay(None, f, f.area()))
        .unwrap();
    assert!(buffer_to_string(&terminal).trim().is_empty());
}

#[test]
fn the_keyboard_screen_draws_the_share_notice_while_it_is_open() {
    let mut screen = crate::KeyboardScreen::new(
        None,
        KeyboardState::default(),
        crate::KeyboardMmlInput::default(),
        crate::KeyboardNoteGuide::new(None),
    );
    screen.share_notice = Some("cmrt kb -t auto".to_string());
    let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
    terminal
        .draw(|f| draw(&mut screen, &crate::KeyboardConnectionStatus::default(), f))
        .unwrap();
    let rendered = buffer_to_string(&terminal);

    assert!(rendered
        .replace(' ', "")
        .contains("クリップボードにコピーしました"));
    assert!(rendered.contains("cmrt kb -t auto"));
    assert!(rendered.contains("y:share"));
}
