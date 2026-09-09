use super::*;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

fn key(code: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(code), KeyModifiers::NONE)
}

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn the_progression_input_is_drawn_as_the_frontmost_overlay_with_the_current_value() {
    let mut screen = screen_with(song_of_one_section());
    screen.handle_key_event(key('i'));

    let rendered = buffer_to_string(&render(&screen));

    assert!(squeeze(&rendered).contains("進行(degrees)"), "{rendered}");
    assert!(rendered.contains("I-V-VIm-IV"), "{rendered}");
    assert!(squeeze(&rendered).contains("Enter:確定"), "{rendered}");
    assert!(rendered.contains("Esc:cancel"), "{rendered}");
}

#[test]
fn the_name_input_shows_the_name_field() {
    let mut screen = screen_with(song_of_one_section());
    screen.handle_key_event(key('n'));

    let rendered = buffer_to_string(&render(&screen));

    assert!(squeeze(&rendered).contains("名前(name)"), "{rendered}");
}

/// 確定できない理由は overlay の中に出す。下段の `error` 行は次のキーで消えるので、
/// 「直すまで出しっぱなし」の用途には使えない。
///
/// degrees も prefix も検証しなくなったので、理由が出る唯一の相手は「空の名前」。
#[test]
fn the_reason_for_a_rejected_commit_is_drawn_inside_the_overlay() {
    let mut screen = screen_with(song_of_one_section());
    screen.handle_key_event(key('n'));
    for _ in 0..64 {
        screen.handle_key_event(plain(KeyCode::Backspace));
    }
    screen.handle_key_event(plain(KeyCode::Enter));

    let rendered = overlay_message_text(&render(&screen));

    assert!(
        rendered.contains(&squeeze(crate::screen::line_input::EMPTY_NAME_MESSAGE)),
        "{rendered}"
    );
    // 下段の `error` は立てていない（overlay 側が出しっぱなしにする役目）。
    assert_eq!(screen.error, None);
}

/// 読めない進行を打っても、理由は出ずにそのまま閉じる（`!` も出ない）。
#[test]
fn a_progression_the_screen_cannot_read_closes_the_overlay_without_a_reason() {
    let mut screen = screen_with(song_of_one_section());
    screen.handle_key_event(key('i'));
    for _ in 0..64 {
        screen.handle_key_event(plain(KeyCode::Backspace));
    }
    for ch in "zzz".chars() {
        screen.handle_key_event(key(ch));
    }
    screen.handle_key_event(plain(KeyCode::Enter));

    let buffer = render(&screen);
    let rendered = buffer_to_string(&buffer);

    assert!(!screen.line_input_open(), "{rendered}");
    // 打った進行は左 pane に出て、`!` は画面のどこにも無い。
    assert!(
        pane_text(&buffer, Pane::Sections).contains("zzz"),
        "{rendered}"
    );
    assert!(!rendered.contains('!'), "{rendered}");
}

/// 折り返した理由は行をまたぐので、枠線ごと落として 1 本の文字列として読む。
/// `squeeze` だけだと行の継ぎ目に `│` が残り、必ず落ちる。
fn overlay_message_text(buffer: &ratatui::buffer::Buffer) -> String {
    squeeze(&buffer_to_string(buffer))
        .chars()
        .filter(|ch| *ch != '│')
        .collect()
}

/// 端末カーソルは入力欄の中に置く（app の `uses_textarea_cursor` と対。
/// 置き忘れると「どこを打っているのか」が画面から分からない）。
#[test]
fn the_terminal_cursor_sits_inside_the_input() {
    let mut screen = screen_with(song_of_one_section());
    screen.handle_key_event(key('i'));

    let mut terminal = Terminal::new(TestBackend::new(TEST_WIDTH, TEST_HEIGHT)).unwrap();
    terminal.draw(|f| draw(&screen, f)).unwrap();

    let position = terminal.get_cursor_position().unwrap();
    assert!(terminal.backend().buffer().area.contains(position));
}

/// 入力欄を閉じたら overlay も消える（描き残しがない）。
#[test]
fn closing_the_input_removes_the_overlay() {
    let mut screen = screen_with(song_of_one_section());
    screen.handle_key_event(key('i'));
    screen.handle_key_event(plain(KeyCode::Esc));

    let rendered = squeeze(&buffer_to_string(&render(&screen)));

    assert!(!rendered.contains("Esc:cancel"), "{rendered}");
    assert!(!rendered.contains("進行(degrees)"), "{rendered}");
}

/// 狭い端末でも overlay を描いて落ちない。
#[test]
fn a_tiny_terminal_still_draws_the_input_overlay() {
    let mut screen = screen_with(song_of_one_section());
    screen.handle_key_event(key('i'));

    for (width, height) in [(1, 1), (4, 3), (20, 6), (40, 10)] {
        render_sized(&screen, width, height);
    }
}

/// 手入力した進行が、確定後に左 pane の行へ出るところまで通す。
#[test]
fn a_hand_typed_progression_appears_in_the_pane_after_the_commit() {
    let mut screen = screen_with(song_of_one_section());
    screen.handle_key_event(key('i'));
    for _ in 0..64 {
        screen.handle_key_event(plain(KeyCode::Backspace));
    }
    for ch in "I-IV-V-I-VIm-IV".chars() {
        screen.handle_key_event(key(ch));
    }
    screen.handle_key_event(plain(KeyCode::Enter));

    let pane = pane_text(&render(&screen), Pane::Sections);
    let rows = content_rows(&pane);

    // 小節数の列を消したぶん degrees の列が広がり、この長さなら丸ごと収まる。
    assert!(rows[0].contains("I-IV-V-I-VIm-IV"), "{pane}");
    assert_eq!(screen.song.sections[0].degrees, "I-IV-V-I-VIm-IV");
}
