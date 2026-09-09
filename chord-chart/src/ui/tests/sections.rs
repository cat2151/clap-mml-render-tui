use super::*;

use cmrt_tui_core::{buffer_test::find_text_ignoring_spaces, theme::cursor_highlight_bg};

use crate::ui::text::ELLIPSIS;

#[test]
fn the_sections_pane_shows_the_name_and_the_progression() {
    let screen = screen_with(song_of_one_section());

    let pane = pane_text(&render(&screen), Pane::Sections);

    assert!(pane.contains(" A "), "{pane}");
    assert!(pane.contains("I-V-VIm-IV"), "{pane}");
    assert!(pane.contains("Sections"), "{pane}");
}

/// 小節数の列は概念ごと消した（何小節ぶんかは進行の記法が持つ）。
#[test]
fn no_row_carries_a_measure_count() {
    let mut song = Song::empty();
    song.push_section("A", "I-V-VIm-IV");
    song.push_section("B", "I-V");

    let pane = pane_text(&render(&screen_with(song)), Pane::Sections);

    assert!(!squeeze(&pane).contains("小節"), "{pane}");
    assert_eq!(content_rows(&pane).len(), 2, "{pane}");
}

/// この画面は degrees を解釈しないので、`!` の列そのものが無い。
/// 読めない進行も、打ったとおりに 1 行として出る。
#[test]
fn a_progression_the_screen_cannot_read_is_shown_without_any_bang() {
    let mut song = Song::empty();
    song.push_section("A", "I-V-VIm-IV");
    song.push_section("Bad", "zzz");

    let pane = pane_text(&render(&screen_with(song)), Pane::Sections);
    let rows = content_rows(&pane);

    let bad_row = rows
        .iter()
        .find(|line| line.contains("Bad"))
        .unwrap_or_else(|| panic!("{pane}"));
    // 進行そのものは打ったとおりに出る。
    assert!(bad_row.contains("zzz"), "{bad_row:?}");
    // `!` の列は pane から消えた（どの行にも出ない）。
    assert!(!pane.contains('!'), "{pane}");
}

/// 枠に収まらない進行は、切れたことが分かる形で切る（chord track と同じ扱い）。
#[test]
fn a_progression_too_long_for_the_pane_ends_with_an_ellipsis() {
    let mut song = Song::empty();
    song.sections.push(long_section(1));
    song.arrangement.push(SectionId::new(1));

    let pane = pane_text(&render(&screen_with(song)), Pane::Sections);
    let row = content_rows(&pane)
        .into_iter()
        .find(|line| line.contains("Long"))
        .unwrap_or_else(|| panic!("{pane}"));
    assert!(row.contains(ELLIPSIS), "{row:?}");
    // 途中までは読める＝ただ消したのではない。
    assert!(row.contains("I-V-VIm"), "{row:?}");
    // 名前の列は切り詰めに巻き込まれない。
    assert!(row.contains("Long"), "{row:?}");
}

/// フォーカスされている pane のカーソル行だけが強調される。
#[test]
fn the_cursor_row_is_highlighted_only_while_the_pane_has_focus() {
    let mut song = Song::empty();
    song.push_section("A", "I-V");
    song.push_section("B", "I-IV");
    let mut screen = screen_with(song);
    screen.section_cursor = 1;

    let buffer = render(&screen);
    // 2 行目にしか無い進行を目印にする（`B` は見出しの BPM にも当たる）。
    let (x, y) = find_text_ignoring_spaces(&buffer, "I-IV");
    let cell = buffer.cell((x, y)).unwrap();
    assert_eq!(cell.bg, cursor_highlight_bg(cell.fg));

    screen.focus = Pane::Arrangement;
    let buffer = render(&screen);
    let cell = buffer.cell((x, y)).unwrap();
    assert_ne!(cell.bg, cursor_highlight_bg(cell.fg));
    // 位置そのものは動かない（どちらの pane にいても場所を見失わない）。
    assert!(
        row_text(&buffer, y).contains('>'),
        "{}",
        buffer_to_string(&buffer)
    );
}

/// カーソルが下へ行っても、その行が必ず描かれる。
#[test]
fn a_cursor_below_the_visible_rows_scrolls_the_pane() {
    let mut song = Song::empty();
    for index in 0..40 {
        song.push_section(format!("S{index}"), "I-V");
    }
    let mut screen = screen_with(song);
    screen.section_cursor = 39;

    let pane = pane_text(&render(&screen), Pane::Sections);

    assert!(pane.contains("S39"), "{pane}");
    assert!(!pane.contains("S0 "), "{pane}");
    assert_eq!(content_rows(&pane).len(), 18, "{pane}");
}

/// キーを押した結果が絵として出るところまで通す。状態のテストだけだと、
/// 追加した行が画面のどこにも出ていなくても緑になる。
#[test]
fn a_section_added_with_the_g_key_appears_in_the_pane_with_the_cursor_on_it() {
    let mut screen = ChordChartScreen::new(song_of_one_section());
    screen.set_chord_progression_source(std::sync::Arc::new(|| vec!["I-IV-V-I".to_string()]));

    screen.handle_key_event(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('g'),
        crossterm::event::KeyModifiers::NONE,
    ));

    let buffer = render(&screen);
    let pane = pane_text(&buffer, Pane::Sections);
    assert!(pane.contains(" B "), "{pane}");
    assert!(pane.contains("I-IV-V-I"), "{pane}");
    // カーソルは足した行（2 行目）。1 行目のままだと `r` / `i` の効き先がずれる。
    let rows = content_rows(&pane);
    assert!(rows[1].starts_with('>'), "{rows:?}");
}

/// 保存ファイルが読めなかったときの自動抽選も、**画面に出る**ところまで通す。
/// 状態だけを見ていると、足した行がどこにも描かれていなくても緑になる。
#[test]
fn the_section_picked_for_an_unloadable_song_appears_in_both_panes() {
    let mut screen = ChordChartScreen::restored(None);
    screen.set_chord_progression_source(std::sync::Arc::new(|| vec!["I-IV-V-I".to_string()]));

    screen.enter();

    let buffer = render(&screen);
    let sections = pane_text(&buffer, Pane::Sections);
    assert!(sections.contains(" A "), "{sections}");
    assert!(sections.contains("I-IV-V-I"), "{sections}");
    // 右 pane も 1 行から始まる（section だけだと並びが空の画面になる）。
    let arrangement = pane_text(&buffer, Pane::Arrangement);
    assert!(arrangement.contains("I-IV-V-I"), "{arrangement}");
    assert_eq!(content_rows(&arrangement).len(), 1, "{arrangement}");
}

/// カタログが無くて抽選できなかったときは、両 pane とも空のまま描ける。
/// 理由は下段に出るので、画面が「無反応」には見えない。
#[test]
fn a_song_that_could_not_be_picked_draws_two_empty_panes_with_a_reason() {
    let mut screen = ChordChartScreen::restored(None);

    screen.enter();

    let buffer = render(&screen);
    assert!(content_rows(&pane_text(&buffer, Pane::Sections)).is_empty());
    assert!(content_rows(&pane_text(&buffer, Pane::Arrangement)).is_empty());
    assert!(
        squeeze(&buffer_to_string(&buffer)).contains(&squeeze(crate::catalog::NO_CATALOG_MESSAGE)),
        "{}",
        buffer_to_string(&buffer)
    );
}
