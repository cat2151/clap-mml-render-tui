use super::*;

use crate::ui::text::ELLIPSIS;

#[test]
fn the_arrangement_pane_lists_every_entry_in_order() {
    let screen = screen_with(song_of_eight_rows());

    let pane = pane_text(&render(&screen), Pane::Arrangement);
    let rows = content_rows(&pane);

    assert_eq!(rows.len(), 8, "{pane}");
    assert!(rows.iter().all(|row| row.contains("I-V")), "{pane}");
    // 同じ section を何度並べてもよい（A が 4 回、B が 4 回）。
    assert_eq!(pane.matches(" A ").count(), 4, "{pane}");
    assert_eq!(pane.matches(" B ").count(), 4, "{pane}");
}

/// 小節数の列も開始時刻の列も出さない（どちらも進行を解釈しないと出せない）。
#[test]
fn no_row_carries_a_measure_count_or_a_start_time() {
    let pane = pane_text(
        &render(&screen_with(song_of_eight_rows())),
        Pane::Arrangement,
    );

    assert!(!squeeze(&pane).contains("小節"), "{pane}");
    assert!(!pane.contains(':'), "{pane}");
}

/// 行番号は `1`..`9` の挿入キーの番号と一致していなければならない。
#[test]
fn rows_are_numbered_from_one_in_arrangement_order() {
    let screen = screen_with(song_of_eight_rows());

    let pane = pane_text(&render(&screen), Pane::Arrangement);
    let rows = content_rows(&pane);

    // 先頭の `>` はカーソル記号。番号はその次。
    assert!(
        rows[0].trim_start_matches('>').starts_with('1'),
        "{:?}",
        rows[0]
    );
    assert!(
        rows[7].trim_start_matches('>').starts_with('8'),
        "{:?}",
        rows[7]
    );
}

/// 編集の途中で参照先が引けなくなっても、行を落とさず番号をずらさない。
#[test]
fn an_entry_without_a_section_still_occupies_its_row() {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V");
    song.arrangement = vec![a, SectionId::new(999), a];

    let pane = pane_text(&render(&screen_with(song)), Pane::Arrangement);
    let rows = content_rows(&pane);

    assert_eq!(rows.len(), 3, "{pane}");
    assert!(rows[1].contains('?'), "{:?}", rows[1]);
    assert!(rows[2].starts_with('3'), "{:?}", rows[2]);
}

#[test]
fn a_progression_too_long_for_the_pane_ends_with_an_ellipsis() {
    let mut song = Song::empty();
    song.sections.push(long_section(1));
    song.arrangement.push(SectionId::new(1));

    let pane = pane_text(&render(&screen_with(song)), Pane::Arrangement);
    let row = content_rows(&pane)
        .into_iter()
        .find(|line| line.contains("Long"))
        .unwrap_or_else(|| panic!("{pane}"));
    assert!(row.contains(ELLIPSIS), "{row:?}");
    // 名前の列は切り詰めに巻き込まれない。
    assert!(row.contains("Long"), "{row:?}");
}

#[test]
fn an_empty_arrangement_draws_an_empty_pane_without_panicking() {
    let mut song = Song::empty();
    song.push_section("A", "I-V");

    let pane = pane_text(&render(&screen_with(song)), Pane::Arrangement);

    assert!(pane.contains("Arrangement"), "{pane}");
    assert!(content_rows(&pane).is_empty(), "{pane}");
}

/// 並べたり動かしたりした結果が絵として出るところまで通す。状態のテストだけだと、
/// 並べ替えた結果が画面のどこにも出ていなくても緑になる。
#[test]
fn the_rows_built_with_the_arrangement_keys_appear_in_the_pane_in_order() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut song = Song::empty();
    song.push_section("A", "I-V-VIm-IV");
    song.push_section("B", "IIm-V-I-VIm");
    let mut screen = screen_with(song);
    screen.focus = Pane::Arrangement;

    // `1` `2` で A B を並べ、`Alt+↓` は末尾なので効かず、`Alt+↑` で B を先頭へ上げる。
    for (code, modifiers) in [
        (KeyCode::Char('1'), KeyModifiers::NONE),
        (KeyCode::Char('2'), KeyModifiers::NONE),
        (KeyCode::Down, KeyModifiers::ALT),
        (KeyCode::Up, KeyModifiers::ALT),
    ] {
        screen.handle_key_event(KeyEvent::new(code, modifiers));
    }

    let pane = pane_text(&render(&screen), Pane::Arrangement);
    let rows = content_rows(&pane);

    assert_eq!(rows.len(), 2, "{pane}");
    assert!(rows[0].contains("IIm-V-I-VIm"), "{rows:?}");
    assert!(rows[1].contains("I-V-VIm-IV"), "{rows:?}");
    // カーソルは動かした行に付いて上がっている。
    assert!(rows[0].starts_with('>'), "{rows:?}");
    // 行番号は並びに追従する（上げた B が 1 行目）。
    assert!(rows[0].trim_start_matches('>').starts_with('1'), "{rows:?}");
    assert!(rows[1].starts_with('2'), "{rows:?}");
}
