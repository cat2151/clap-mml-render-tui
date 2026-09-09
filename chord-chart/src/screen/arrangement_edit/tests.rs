//! Arrangement pane の編集を、実際のキー（`1`..`9` / `dd` / `Alt+↑` / `Alt+↓`）から確かめる。
//!
//! 抽選が絡まないので結果は決め打ちできる。見るのは
//! 「並びそのもの」「カーソルがどの行に残るか」「保存を要求したか」の 3 つ。

use super::*;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{Pane, Song};

fn key(code: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(code), KeyModifiers::NONE)
}

/// `Alt+↑` / `Alt+↓` は `KeyModifiers::ALT` 付きで届く。実際の来方で試す。
fn alt(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::ALT)
}

/// `dd` は 2 打。1 打目では何も起きないことも一緒に押さえる。
fn press_dd(screen: &mut ChordChartScreen) -> ChordChartAction {
    assert_eq!(
        screen.handle_key_event(key('d')),
        ChordChartAction::Continue
    );
    screen.handle_key_event(key('d'))
}

/// section 3 つ（A / B / Sabi）を持ち、arrangement は空の画面。
/// フォーカスは Arrangement pane に置いてある。
fn screen_with_three_sections() -> ChordChartScreen {
    let mut song = Song::empty();
    song.push_section("A", "I-V-VIm-IV");
    song.push_section("B", "IIm-V-I-VIm");
    song.push_section("Sabi", "IV-V-IIIm-VIm");
    let mut screen = ChordChartScreen::new(song);
    screen.focus = Pane::Arrangement;
    screen
}

/// arrangement を「並んだ section 名」として読む。id を直接見るより読みやすい。
fn arranged_names(screen: &ChordChartScreen) -> Vec<String> {
    screen
        .song
        .arrangement
        .iter()
        .map(|id| {
            screen
                .song
                .section(*id)
                .map_or_else(|| "?".to_string(), |section| section.name.clone())
        })
        .collect()
}

#[test]
fn a_digit_on_an_empty_arrangement_puts_the_first_entry_under_the_cursor() {
    let mut screen = screen_with_three_sections();

    assert_eq!(
        screen.handle_key_event(key('1')),
        ChordChartAction::SongChanged
    );

    assert_eq!(arranged_names(&screen), vec!["A"]);
    assert_eq!(screen.clamped_arrangement_cursor(), 0);
    assert_eq!(screen.error, None);
}

/// 番号は**左 pane の行番号**（section の並び順）であって、右 pane の行番号ではない。
#[test]
fn a_digit_inserts_the_section_of_that_row_in_the_sections_pane() {
    let mut screen = screen_with_three_sections();

    screen.handle_key_event(key('2'));
    screen.handle_key_event(key('3'));

    assert_eq!(arranged_names(&screen), vec!["B", "Sabi"]);
}

/// 挿入はカーソルの**次**。末尾へ足すのではない。
#[test]
fn a_digit_inserts_after_the_cursor_and_moves_the_cursor_onto_it() {
    let mut screen = screen_with_three_sections();
    screen.handle_key_event(key('1'));
    screen.handle_key_event(key('1'));
    screen.handle_key_event(key('1'));
    // A A A のうち先頭へ戻ってから B を挿す。
    screen.handle_key_event(key('k'));
    screen.handle_key_event(key('k'));
    assert_eq!(screen.clamped_arrangement_cursor(), 0);

    screen.handle_key_event(key('2'));

    assert_eq!(arranged_names(&screen), vec!["A", "B", "A", "A"]);
    assert_eq!(screen.clamped_arrangement_cursor(), 1);
}

/// section が 3 つのときの `5`。何も起きず、`error` にも出さない（資料 5 章 Stage 6）。
#[test]
fn a_digit_without_a_section_does_nothing_and_says_nothing() {
    let mut screen = screen_with_three_sections();
    screen.handle_key_event(key('1'));

    assert_eq!(
        screen.handle_key_event(key('5')),
        ChordChartAction::Continue
    );

    assert_eq!(arranged_names(&screen), vec!["A"]);
    assert_eq!(screen.error, None);
}

/// `1`..`9` で挿せるのは左 pane の 9 行目まで。10 番目は数字キーでは届かない。
#[test]
fn only_the_first_nine_sections_are_reachable_from_the_digits() {
    let mut song = Song::empty();
    for index in 1..=10 {
        song.push_section(format!("S{index}"), "I-V");
    }
    let mut screen = ChordChartScreen::new(song);
    screen.focus = Pane::Arrangement;

    screen.handle_key_event(key('9'));
    assert_eq!(arranged_names(&screen), vec!["S9"]);

    // `0` は 10 番目の section を指すキーではない（4.5 に無い）。
    assert_eq!(
        screen.handle_key_event(key('0')),
        ChordChartAction::Continue
    );
    assert_eq!(arranged_names(&screen), vec!["S9"]);
}

#[test]
fn dd_removes_only_the_row_and_leaves_the_section_alone() {
    let mut screen = screen_with_three_sections();
    screen.handle_key_event(key('1'));
    screen.handle_key_event(key('1'));

    assert_eq!(press_dd(&mut screen), ChordChartAction::SongChanged);

    assert_eq!(arranged_names(&screen), vec!["A"]);
    // 素材は消えない。消すのは左 pane の `dd` の仕事。
    assert_eq!(screen.song.sections.len(), 3);
}

/// 末尾を消したあと、カーソルは実在する行へ戻っていること。書き戻さないと
/// 直後の `k` が「見た目は動かないのに index だけ減る」無反応の 1 回になる。
#[test]
fn deleting_the_last_row_pulls_the_cursor_back_onto_a_real_row() {
    let mut screen = screen_with_three_sections();
    screen.handle_key_event(key('1'));
    screen.handle_key_event(key('2'));
    assert_eq!(screen.arrangement_cursor, 1);

    press_dd(&mut screen);

    assert_eq!(screen.arrangement_cursor, 0);
    assert_eq!(arranged_names(&screen), vec!["A"]);
}

#[test]
fn deleting_the_only_row_empties_the_arrangement_without_panicking() {
    let mut screen = screen_with_three_sections();
    screen.handle_key_event(key('1'));

    press_dd(&mut screen);
    // 空になったあとにもう一度押しても落ちないし、保存も要求しない。
    assert_eq!(press_dd(&mut screen), ChordChartAction::Continue);

    assert!(screen.song.arrangement.is_empty());
    assert_eq!(screen.arrangement_cursor, 0);
}

#[test]
fn alt_up_and_down_move_the_row_and_the_cursor_together() {
    let mut screen = screen_with_three_sections();
    screen.handle_key_event(key('1'));
    screen.handle_key_event(key('2'));
    screen.handle_key_event(key('3'));
    screen.handle_key_event(key('k'));
    assert_eq!(screen.clamped_arrangement_cursor(), 1);

    assert_eq!(
        screen.handle_key_event(alt(KeyCode::Down)),
        ChordChartAction::SongChanged
    );
    assert_eq!(arranged_names(&screen), vec!["A", "Sabi", "B"]);
    assert_eq!(screen.clamped_arrangement_cursor(), 2);

    screen.handle_key_event(alt(KeyCode::Up));
    screen.handle_key_event(alt(KeyCode::Up));
    assert_eq!(arranged_names(&screen), vec!["B", "A", "Sabi"]);
    assert_eq!(screen.clamped_arrangement_cursor(), 0);
}

/// 端で押しても範囲外にならず、曲も変わらない＝保存も要求しない。
#[test]
fn moving_past_either_end_changes_nothing() {
    let mut screen = screen_with_three_sections();
    screen.handle_key_event(key('1'));
    screen.handle_key_event(key('2'));

    // 末尾で `Alt+↓`。
    assert_eq!(
        screen.handle_key_event(alt(KeyCode::Down)),
        ChordChartAction::Continue
    );
    assert_eq!(arranged_names(&screen), vec!["A", "B"]);
    assert_eq!(screen.clamped_arrangement_cursor(), 1);

    // 先頭で `Alt+↑`。
    screen.handle_key_event(key('k'));
    assert_eq!(
        screen.handle_key_event(alt(KeyCode::Up)),
        ChordChartAction::Continue
    );
    assert_eq!(arranged_names(&screen), vec!["A", "B"]);
    assert_eq!(screen.clamped_arrangement_cursor(), 0);
}

/// 空の arrangement では削除も移動も無反応（panic しない）。
#[test]
fn the_arrangement_keys_are_inert_while_the_pane_is_empty() {
    let mut screen = screen_with_three_sections();

    assert_eq!(press_dd(&mut screen), ChordChartAction::Continue);
    for event in [alt(KeyCode::Down), alt(KeyCode::Up)] {
        assert_eq!(screen.handle_key_event(event), ChordChartAction::Continue);
    }

    assert!(screen.song.arrangement.is_empty());
}

/// 左 pane にフォーカスがある間は、右 pane のキー（`1`）が効かないこと。
///
/// `dd` と `Alt+↑↓` をここで試さないのは、左 pane では section の削除・並べ替えという
/// **別の意味を持つ**共通キーだから（左 pane 側は `screen/section_edit/tests.rs`）。
#[test]
fn the_arrangement_keys_are_inert_while_the_sections_pane_has_focus() {
    let mut screen = screen_with_three_sections();
    screen.handle_key_event(key('1'));
    screen.handle_key_event(key('2'));
    let before = screen.song.arrangement.clone();

    screen.focus = Pane::Sections;
    assert_eq!(
        screen.handle_key_event(key('1')),
        ChordChartAction::Continue
    );

    assert_eq!(screen.song.arrangement, before);
    assert_eq!(screen.song.sections.len(), 3);
}

/// 一連の操作のあと、並びが押した順そのものになっていること。
/// 挿入・並べ替え・削除を混ぜて、行数と順序の両方を見る。
#[test]
fn the_arrangement_follows_the_keys_after_a_round_of_editing() {
    let mut screen = screen_with_three_sections();
    screen.handle_key_event(key('1'));
    screen.handle_key_event(key('2'));
    screen.handle_key_event(key('3'));
    assert_eq!(arranged_names(&screen), vec!["A", "B", "Sabi"]);

    // 同じ section をもう 1 回並べられる。
    screen.handle_key_event(key('3'));
    assert_eq!(arranged_names(&screen), vec!["A", "B", "Sabi", "Sabi"]);

    // 並べ替えても行数は変わらない（順序だけの操作）。
    screen.handle_key_event(alt(KeyCode::Up));
    assert_eq!(screen.song.arrangement.len(), 4);
    assert_eq!(arranged_names(&screen), vec!["A", "B", "Sabi", "Sabi"]);

    // 1 行消す。
    press_dd(&mut screen);
    assert_eq!(arranged_names(&screen), vec!["A", "B", "Sabi"]);
}

/// 左 pane を並べ替えると、`1`..`9` の指す先も並びに追従する。
#[test]
fn the_digits_follow_the_order_of_the_sections_pane() {
    let mut screen = screen_with_three_sections();
    screen.focus = Pane::Sections;
    // A B Sabi → B A Sabi。
    screen.handle_key_event(alt(KeyCode::Down));
    screen.focus = Pane::Arrangement;

    screen.handle_key_event(key('1'));

    // 1 行目が B になったので、`1` は B を挿す。
    assert_eq!(arranged_names(&screen), vec!["B"]);
}
