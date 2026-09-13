//! `n` の section 名入力を、実際のキーの並びから確かめる。

use super::*;

#[test]
fn n_opens_the_input_prefilled_with_the_current_name() {
    let mut screen = two_section_screen();
    screen.section_cursor = 1;

    screen.handle_key_event(key('n'));

    assert_eq!(screen.line_input().unwrap().target(), LineInputTarget::Name);
    assert_eq!(input_value(&screen), "B");
}

/// 端末によっては `Enter` が `Ctrl+M` として届く。両方で確定できること。
#[test]
fn control_m_commits_just_like_enter() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);
    type_text(&mut screen, "Sabi");

    assert_eq!(
        screen.handle_key_event(ctrl('m')),
        ChordChartAction::SongChanged
    );
    assert!(!screen.line_input_open());
    assert_eq!(screen.song.sections[0].name, "Sabi");
}

/// 理由が出たあと文字を直したら、その場で理由は消える（古い理由が残らない）。
#[test]
fn typing_after_a_rejected_commit_clears_the_reason() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);
    screen.handle_key_event(plain(KeyCode::Enter));
    assert!(screen.line_input().unwrap().error().is_some());

    type_text(&mut screen, "S");

    assert!(screen.line_input().unwrap().error().is_none());
}

/// 直してから確定し直せる＝入力欄が「直す場所」として機能している。
#[test]
fn a_rejected_name_can_be_fixed_without_reopening_the_input() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);
    screen.handle_key_event(plain(KeyCode::Enter));

    type_text(&mut screen, "Sabi");

    assert_eq!(
        screen.handle_key_event(plain(KeyCode::Enter)),
        ChordChartAction::SongChanged
    );
    assert!(!screen.line_input_open());
    assert_eq!(screen.song.sections[0].name, "Sabi");
}

/// 空の名前では確定させない（左 pane の行が読めなくなる）。
#[test]
fn an_empty_name_keeps_the_input_open() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);

    assert_eq!(
        screen.handle_key_event(plain(KeyCode::Enter)),
        ChordChartAction::Continue
    );

    assert!(screen.line_input_open());
    assert_eq!(
        screen.line_input().unwrap().error(),
        Some(EMPTY_NAME_MESSAGE)
    );
    assert_eq!(screen.song.sections[0].name, "A");
}

/// 空白だけの名前も空と同じ（見た目は空行になる）。
#[test]
fn a_whitespace_only_name_counts_as_empty() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);
    type_text(&mut screen, "   ");

    screen.handle_key_event(plain(KeyCode::Enter));

    assert!(screen.line_input_open());
    assert_eq!(screen.song.sections[0].name, "A");
}

/// 前後の空白は落として確定する。
#[test]
fn the_committed_value_is_trimmed() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);
    type_text(&mut screen, "  Sabi  ");
    screen.handle_key_event(plain(KeyCode::Enter));

    assert_eq!(screen.song.sections[0].name, "Sabi");
}

#[test]
fn escape_closes_the_input_and_keeps_the_old_value() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);
    type_text(&mut screen, "Intro");

    assert_eq!(
        screen.handle_key_event(plain(KeyCode::Esc)),
        ChordChartAction::Continue
    );

    assert!(!screen.line_input_open());
    assert_eq!(screen.song.sections[0].name, "A");
}

/// 値が変わらない確定でファイルを書き直さない（`Continue` を返す）。
#[test]
fn committing_the_same_value_does_not_ask_for_a_save() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));

    assert_eq!(
        screen.handle_key_event(plain(KeyCode::Enter)),
        ChordChartAction::Continue
    );
    assert!(!screen.line_input_open());
    assert_eq!(screen.song.sections[0].name, "A");
}

/// 開いている間は他のキーが裏の画面に届かない。`?` も `Tab` も文字として入る。
#[test]
fn the_input_swallows_every_other_key() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);

    screen.handle_key_event(key('?'));
    screen.handle_key_event(plain(KeyCode::Tab));
    screen.handle_key_event(key('j'));

    assert!(!screen.help_open);
    assert_eq!(screen.focus, Pane::Sections);
    assert_eq!(screen.section_cursor, 0);
    assert!(screen.line_input_open());
}

/// `Ctrl+W`（単語削除）が効く＝自前の push/pop ではなく textarea を通している。
#[test]
fn the_textarea_keybinds_such_as_control_w_are_available() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);
    type_text(&mut screen, "Sabi Verse");

    screen.handle_key_event(ctrl('w'));

    assert_eq!(input_value(&screen), "Sabi ");
}

/// section が 1 つも無いときは開かない（書き込む先が無い）。
#[test]
fn n_does_nothing_without_a_section() {
    let mut screen = ChordChartScreen::new(Song::empty());

    assert_eq!(
        screen.handle_key_event(key('n')),
        ChordChartAction::Continue
    );
    assert!(!screen.line_input_open());
}

/// Arrangement pane では `n` が効かない（左 pane 専用のキー）。
#[test]
fn n_is_inert_while_the_arrangement_pane_has_focus() {
    let mut screen = two_section_screen();
    screen.focus = Pane::Arrangement;

    assert_eq!(
        screen.handle_key_event(key('n')),
        ChordChartAction::Continue
    );
    assert!(!screen.line_input_open());
}

/// カーソルを動かしてから開くと、その section が対象になる。
#[test]
fn the_input_edits_the_section_under_the_cursor() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('j'));

    screen.handle_key_event(key('n'));
    clear_input(&mut screen);
    type_text(&mut screen, "Sabi");
    screen.handle_key_event(plain(KeyCode::Enter));

    assert_eq!(screen.song.sections[0].name, "A");
    assert_eq!(screen.song.sections[1].name, "Sabi");
}
