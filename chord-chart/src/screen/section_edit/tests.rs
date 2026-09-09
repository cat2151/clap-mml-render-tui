//! Sections pane の編集を、実際のキー（`g` / `r` / `dd` / `Alt+↑↓`）から確かめる。
//!
//! 抽選は乱数を通るので、**カタログの中身を固定して**結果を決め打ちにする
//! （固定カタログ 1 件なら引けるものは 1 つしかない）。

use super::*;

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::catalog::NO_CATALOG_MESSAGE;
use crate::{Pane, Song};

fn key(code: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(code), KeyModifiers::NONE)
}

/// `dd` は 2 打。1 打目では何も起きないことも一緒に押さえる。
fn press_dd(screen: &mut ChordChartScreen) -> ChordChartAction {
    assert_eq!(
        screen.handle_key_event(key('d')),
        ChordChartAction::Continue
    );
    screen.handle_key_event(key('d'))
}

/// `Alt+↑` / `Alt+↓` は ALT 付きで届く。実際の来方で押す。
fn alt(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::ALT)
}

/// カタログを注入した画面。`progressions` が抽選の全候補。
fn screen_with_catalog(song: Song, progressions: &[&str]) -> ChordChartScreen {
    let progressions: Vec<String> = progressions
        .iter()
        .map(|text| (*text).to_string())
        .collect();
    let mut screen = ChordChartScreen::new(song);
    screen.set_chord_progression_source(Arc::new(move || progressions.clone()));
    screen
}

/// section 1 つ（`A`）を 1 回だけ並べた曲。既定の曲は無くなったので、
/// 「1 つある状態」が要るテストはここから始める。
fn one_section_song() -> Song {
    let mut song = Song::empty();
    let id = song.push_section("A", "I-V-VIm-IV");
    song.arrangement = vec![id];
    song
}

fn section_names(screen: &ChordChartScreen) -> Vec<String> {
    screen
        .song
        .sections
        .iter()
        .map(|section| section.name.clone())
        .collect()
}

#[test]
fn g_adds_a_section_from_the_catalog_and_names_it_in_sequence() {
    let mut screen = screen_with_catalog(one_section_song(), &["I-IV-V-I"]);

    assert_eq!(
        screen.handle_key_event(key('g')),
        ChordChartAction::SongChanged
    );

    assert_eq!(section_names(&screen), vec!["A", "B"]);
    assert_eq!(screen.song.sections[1].degrees, "I-IV-V-I");
    // 足した行へカーソルが降りる（直後の `r` / `i` がそこに効く）。
    assert_eq!(screen.clamped_section_cursor(), 1);
    assert_eq!(screen.error, None);
}

/// arrangement へは並べない（並べるのは Arrangement pane の `1`..`9`）。
#[test]
fn g_does_not_touch_the_arrangement() {
    let mut screen = screen_with_catalog(one_section_song(), &["I-IV-V-I"]);
    let before = screen.song.arrangement.clone();

    screen.handle_key_event(key('g'));

    assert_eq!(screen.song.arrangement, before);
}

#[test]
fn the_automatic_names_go_through_the_alphabet_and_then_start_numbering() {
    let mut screen = screen_with_catalog(Song::empty(), &["I-IV-V-I"]);

    for _ in 0..27 {
        screen.handle_key_event(key('g'));
    }

    let names = section_names(&screen);
    assert_eq!(names.first().map(String::as_str), Some("A"));
    assert_eq!(names.get(25).map(String::as_str), Some("Z"));
    // 26 個で使い切ったので、27 個目は数字付きへ回る。
    assert_eq!(names.get(26).map(String::as_str), Some("A2"));
    let unique: std::collections::BTreeSet<&String> = names.iter().collect();
    assert_eq!(unique.len(), names.len(), "{names:?}");
}

/// 数の数え上げではなく未使用の名前を探す。`B` を消してから足しても `C` が 2 つにならない。
#[test]
fn g_reuses_a_freed_name_instead_of_duplicating_an_existing_one() {
    let mut song = Song::empty();
    song.push_section("A", "I-V-VIm-IV");
    song.push_section("B", "I-V-VIm-IV");
    song.push_section("C", "I-V-VIm-IV");
    let mut screen = screen_with_catalog(song, &["I-IV-V-I"]);
    // `B` を消してから足す。
    screen.handle_key_event(key('j'));
    press_dd(&mut screen);

    screen.handle_key_event(key('g'));

    assert_eq!(section_names(&screen), vec!["A", "C", "B"]);
}

#[test]
fn an_empty_catalog_says_there_is_no_data_instead_of_doing_nothing() {
    let mut screen = screen_with_catalog(one_section_song(), &[]);

    assert_eq!(
        screen.handle_key_event(key('g')),
        ChordChartAction::Continue
    );

    assert_eq!(screen.song.sections.len(), 1);
    assert_eq!(screen.error.as_deref(), Some(NO_CATALOG_MESSAGE));
}

/// カタログを注入していない画面（app のテストと、注入前の一瞬）も同じ扱い。
#[test]
fn a_screen_without_an_injected_catalog_reports_the_same_reason() {
    let mut screen = ChordChartScreen::new(one_section_song());

    screen.handle_key_event(key('g'));

    assert_eq!(screen.song.sections.len(), 1);
    assert_eq!(screen.error.as_deref(), Some(NO_CATALOG_MESSAGE));
}

/// カタログの中身は**そのまま**採る。この画面は degrees を解釈しないので、
/// 「読めない進行だから引き直す」という判断そのものを持たない。
#[test]
fn a_progression_the_screen_cannot_read_is_added_as_it_is() {
    let mut screen = screen_with_catalog(one_section_song(), &["zzz"]);

    assert_eq!(
        screen.handle_key_event(key('g')),
        ChordChartAction::SongChanged
    );

    assert_eq!(screen.song.sections.len(), 2);
    assert_eq!(screen.song.sections[1].degrees, "zzz");
    assert_eq!(screen.error, None);
}

#[test]
fn r_replaces_only_the_progression_of_the_section_under_the_cursor() {
    let mut song = Song::empty();
    song.push_section("A", "I-V-VIm-IV");
    let b = song.push_section("B", "I-V-VIm-IV");
    song.arrangement = vec![b];
    let mut screen = screen_with_catalog(song, &["I-IV-V-I-IIm-V"]);
    screen.section_cursor = 1;

    assert_eq!(
        screen.handle_key_event(key('r')),
        ChordChartAction::SongChanged
    );

    assert_eq!(screen.song.sections[0].degrees, "I-V-VIm-IV");
    assert_eq!(screen.song.sections[1].degrees, "I-IV-V-I-IIm-V");
    // 名前は保つ。arrangement は id を持っているので並びも変わらない。
    assert_eq!(screen.song.sections[1].name, "B");
    assert_eq!(screen.song.arrangement, vec![b]);
}

#[test]
fn r_on_an_empty_catalog_keeps_the_progression_and_shows_the_reason() {
    let mut screen = screen_with_catalog(one_section_song(), &[]);

    assert_eq!(
        screen.handle_key_event(key('r')),
        ChordChartAction::Continue
    );

    assert_eq!(screen.song.sections[0].degrees, "I-V-VIm-IV");
    assert_eq!(screen.error.as_deref(), Some(NO_CATALOG_MESSAGE));
}

/// section が 1 つも無いときに押しても落ちない（`dd` で全部消したあとの状態）。
#[test]
fn the_editing_keys_do_nothing_on_an_empty_sections_pane() {
    let mut screen = screen_with_catalog(Song::empty(), &["I-IV-V-I"]);

    assert_eq!(
        screen.handle_key_event(key('r')),
        ChordChartAction::Continue
    );
    assert_eq!(press_dd(&mut screen), ChordChartAction::Continue);
    for code in [KeyCode::Up, KeyCode::Down] {
        assert_eq!(
            screen.handle_key_event(alt(code)),
            ChordChartAction::Continue
        );
    }

    assert!(screen.song.sections.is_empty());
    assert_eq!(screen.section_cursor, 0);
}

#[test]
fn dd_removes_the_section_and_every_reference_to_it_in_the_arrangement() {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    let b = song.push_section("B", "IIm-V-I-VIm");
    song.arrangement = vec![a, b, a, b];
    let mut screen = ChordChartScreen::new(song);
    assert_eq!(screen.song.arrangement.len(), 4);

    assert_eq!(press_dd(&mut screen), ChordChartAction::SongChanged);

    assert_eq!(section_names(&screen), vec!["B"]);
    assert_eq!(screen.song.arrangement, vec![b, b]);
}

/// 末尾を消すとカーソルがはみ出す。丸めた値を書き戻していないと、次の `k` が
/// 「見た目は動かないのに index だけ減る」無反応の 1 回になる。
#[test]
fn deleting_the_last_row_pulls_the_cursor_back_onto_a_real_row() {
    let mut song = Song::empty();
    song.push_section("A", "I-V-VIm-IV");
    song.push_section("B", "IIm-V-I-VIm");
    song.push_section("C", "IV-V-IIIm-VIm");
    let mut screen = ChordChartScreen::new(song);
    screen.section_cursor = 2;

    press_dd(&mut screen);

    assert_eq!(screen.section_cursor, 1);
    assert_eq!(
        screen
            .selected_section()
            .map(|section| section.name.clone()),
        Some("B".to_string())
    );

    screen.handle_key_event(key('k'));
    assert_eq!(screen.section_cursor, 0);
}

/// 右 pane の参照が全部消えると arrangement も空になる。そのカーソルも丸める。
#[test]
fn deleting_the_only_section_empties_the_arrangement_without_panicking() {
    let mut screen = ChordChartScreen::new(one_section_song());

    press_dd(&mut screen);

    assert!(screen.song.sections.is_empty());
    assert!(screen.song.arrangement.is_empty());
    assert_eq!(screen.clamped_arrangement_cursor(), 0);
}

/// Arrangement pane では Sections の編集キーが効かない。
///
/// `dd` をここで試さないのは、右 pane では「カーソル行を削除」という**別の意味を持つ**から
/// （`screen/arrangement_edit/tests.rs` の
/// `dd_removes_only_the_row_and_leaves_the_section_alone` が右 pane 側を見ている）。
/// section が消えないことは同じテストが `sections.len()` で押さえている。
#[test]
fn the_sections_keys_are_inert_while_the_arrangement_pane_has_focus() {
    let mut screen = screen_with_catalog(one_section_song(), &["I-IV-V-I"]);
    screen.focus = Pane::Arrangement;
    let before = screen.song.clone();

    for code in ['g', 'r', 'i', 'n'] {
        assert_eq!(
            screen.handle_key_event(key(code)),
            ChordChartAction::Continue
        );
    }

    assert_eq!(screen.song, before);
    assert_eq!(screen.error, None);
    // `i` / `n` の入力欄も開かない（開くと以降のキーを全部食ってしまう）。
    assert!(!screen.line_input_open());
}

/// `Alt+↓` は section の並びだけを変える。arrangement（id の並び）は動かない。
#[test]
fn alt_down_reorders_the_sections_without_touching_the_arrangement() {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    let b = song.push_section("B", "IIm-V-I-VIm");
    song.push_section("C", "IV-V-IIIm-VIm");
    song.arrangement = vec![a, b];
    let mut screen = ChordChartScreen::new(song);

    assert_eq!(
        screen.handle_key_event(alt(KeyCode::Down)),
        ChordChartAction::SongChanged
    );

    assert_eq!(section_names(&screen), vec!["B", "A", "C"]);
    // 曲の並びは id 参照なので変わらない。
    assert_eq!(screen.song.arrangement, vec![a, b]);
    // 動かした行にカーソルが付いていく。
    assert_eq!(screen.clamped_section_cursor(), 1);

    assert_eq!(
        screen.handle_key_event(alt(KeyCode::Up)),
        ChordChartAction::SongChanged
    );
    assert_eq!(section_names(&screen), vec!["A", "B", "C"]);
    assert_eq!(screen.clamped_section_cursor(), 0);
}

/// 端で押しても曲は変わらない＝保存も要求しない。
#[test]
fn moving_a_section_past_either_end_changes_nothing() {
    let mut screen = screen_with_catalog(one_section_song(), &["I-IV-V-I"]);
    screen.handle_key_event(key('g'));
    let before = screen.song.clone();

    // 末尾で `Alt+↓`。
    assert_eq!(
        screen.handle_key_event(alt(KeyCode::Down)),
        ChordChartAction::Continue
    );
    // 先頭で `Alt+↑`。
    screen.handle_key_event(key('k'));
    assert_eq!(
        screen.handle_key_event(alt(KeyCode::Up)),
        ChordChartAction::Continue
    );

    assert_eq!(screen.song, before);
}
