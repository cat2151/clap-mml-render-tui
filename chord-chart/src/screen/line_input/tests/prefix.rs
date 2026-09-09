//! `b` の 1 行入力（曲頭の chord2mml 指定）を、実際のキーの並びから確かめる。
//!
//! **入力は一切検証しない**のがこの欄の仕様（資料 7 章 3 の `[user]` 回答）。
//! 「読めない綴りを弾く」テストをここへ足してはいけない。

use super::*;

/// prefix は section を見ないので、section が 0 個の曲でも開く。
fn empty_screen() -> ChordChartScreen {
    ChordChartScreen::new(Song::empty())
}

#[test]
fn b_opens_the_input_prefilled_with_the_current_prefix() {
    let mut screen = two_section_screen();

    assert_eq!(
        screen.handle_key_event(key('b')),
        ChordChartAction::Continue,
        "開いただけでは曲は変わらない＝保存もしない"
    );

    assert!(screen.line_input_open());
    assert_eq!(input_value(&screen), "Key=C BPM120");
}

#[test]
fn enter_commits_the_typed_prefix_and_asks_for_a_save() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('b'));
    clear_input(&mut screen);
    type_text(&mut screen, "Key=A BPM90");

    assert_eq!(
        screen.handle_key_event(plain(KeyCode::Enter)),
        ChordChartAction::SongChanged
    );

    assert!(!screen.line_input_open());
    assert_eq!(screen.song.prefix, "Key=A BPM90");
}

/// `Ctrl+M` も確定キー（端末によっては `Enter` がこちらで届く）。
#[test]
fn ctrl_m_commits_the_prefix_too() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('b'));
    clear_input(&mut screen);
    type_text(&mut screen, "Key=D");

    assert_eq!(
        screen.handle_key_event(ctrl('m')),
        ChordChartAction::SongChanged
    );

    assert_eq!(screen.song.prefix, "Key=D");
}

#[test]
fn esc_throws_the_typed_prefix_away() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('b'));
    clear_input(&mut screen);
    type_text(&mut screen, "Key=A BPM90");

    assert_eq!(
        screen.handle_key_event(plain(KeyCode::Esc)),
        ChordChartAction::Continue
    );

    assert!(!screen.line_input_open());
    assert_eq!(screen.song.prefix, "Key=C BPM120");
}

/// 何を打っても確定する。読めない綴りも、コロン付きの BPM も、空文字も。
#[test]
fn any_text_at_all_is_accepted_without_a_reason_being_shown() {
    for text in ["Key:H BPM:0", "zzz", "", "まったくの自由文"] {
        let mut screen = two_section_screen();
        screen.handle_key_event(key('b'));
        clear_input(&mut screen);
        type_text(&mut screen, text);
        screen.handle_key_event(plain(KeyCode::Enter));

        assert!(!screen.line_input_open(), "{text:?} で開いたまま止まった");
        assert_eq!(screen.song.prefix, text);
        assert_eq!(screen.error, None, "{text:?}");
    }
}

/// 同じ値で確定したら保存しない（曲は変わっていない）。
#[test]
fn committing_the_same_prefix_does_not_ask_for_a_save() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('b'));

    assert_eq!(
        screen.handle_key_event(plain(KeyCode::Enter)),
        ChordChartAction::Continue
    );
}

/// `b` は**共通キー**。右 pane にフォーカスがあっても、section が 0 個でも開く。
#[test]
fn b_works_in_both_panes_and_even_without_a_single_section() {
    for focus in [Pane::Sections, Pane::Arrangement] {
        let mut screen = empty_screen();
        screen.focus = focus;

        screen.handle_key_event(key('b'));

        assert!(screen.line_input_open(), "{focus:?} で開かなかった");
        assert_eq!(input_value(&screen), "Key=C BPM120");
    }
}

/// 入力欄が開いている間は `Ctrl+W`（単語削除）が入力欄へ届く。
/// CONTROL ガードより手前で overlay を見ていないと、ここで無反応になる。
#[test]
fn ctrl_w_reaches_the_prefix_input_instead_of_being_swallowed_by_the_guard() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('b'));

    screen.handle_key_event(ctrl('w'));

    assert!(screen.line_input_open());
    assert_ne!(
        input_value(&screen),
        "Key=C BPM120",
        "単語削除が効いていない"
    );
}

/// `dd` の 1 打目のあとに `b` を押したら、保留は捨てて `b` が普通に効く
/// （押したのに何も起きない 1 回を作らない）。
#[test]
fn a_pending_d_does_not_swallow_the_b_key() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('d'));

    screen.handle_key_event(key('b'));

    assert!(screen.line_input_open());
    assert_eq!(screen.song.sections.len(), 2, "section は消えていない");
}
