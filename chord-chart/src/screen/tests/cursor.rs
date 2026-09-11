//! カーソルと pane フォーカスの移動。`q` もここ（どちらの pane からでも同じ）。
//!
//! pane 移動は `Tab` のトグル。`h` / `l` は行内の chord 移動なので、
//! そちらは `screen/chord_cursor/tests.rs` が見る。

use super::*;

/// `Tab` は 2 pane のトグル。押すたびに行き先が入れ替わる。
#[test]
fn tab_toggles_the_focus_between_the_two_panes() {
    let mut screen = three_section_screen();

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Tab)),
        ChordChartAction::Continue
    );
    assert_eq!(screen.focus, Pane::Arrangement);

    screen.handle_key_event(key(KeyCode::Tab));
    assert_eq!(screen.focus, Pane::Sections);

    screen.handle_key_event(key(KeyCode::Tab));
    assert_eq!(screen.focus, Pane::Arrangement);
}

/// `h` / `l` は pane を動かさない（行内の chord 移動へ役目が変わった）。
#[test]
fn h_and_l_no_longer_move_the_focus() {
    let mut screen = three_section_screen();

    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    assert_eq!(screen.focus, Pane::Sections);

    screen.handle_key_event(key(KeyCode::Tab));
    screen.handle_key_event(key(KeyCode::Char('h')));
    assert_eq!(screen.focus, Pane::Arrangement);
}

/// `q` はどちらの pane からでもアプリ終了。曲は変えない。
#[test]
fn q_asks_the_runtime_to_quit_from_either_pane() {
    let mut screen = three_section_screen();
    let before = screen.song.clone();

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('q'))),
        ChordChartAction::Quit
    );

    screen.focus = Pane::Arrangement;
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('q'))),
        ChordChartAction::Quit
    );
    assert_eq!(screen.song, before);
}

/// `PgDn` / `PgUp` は 10 行。端で止まり、行き過ぎても反対側へは回らない。
#[test]
fn the_page_keys_move_ten_rows_and_stop_at_the_ends() {
    let mut screen = twenty_five_section_screen();

    screen.handle_key_event(key(KeyCode::PageDown));
    assert_eq!(screen.clamped_section_cursor(), 10);

    screen.handle_key_event(key(KeyCode::PageDown));
    assert_eq!(screen.clamped_section_cursor(), 20);

    // 3 回目は末尾（24 行目）で止まる。
    screen.handle_key_event(key(KeyCode::PageDown));
    assert_eq!(screen.clamped_section_cursor(), 24);

    screen.handle_key_event(key(KeyCode::PageUp));
    assert_eq!(screen.clamped_section_cursor(), 14);

    for _ in 0..5 {
        screen.handle_key_event(key(KeyCode::PageUp));
    }
    assert_eq!(screen.clamped_section_cursor(), 0);
}

/// `PgDn` / `PgUp` もフォーカスしている pane だけを動かす。
#[test]
fn the_page_keys_follow_the_focused_pane_only() {
    let mut screen = twenty_five_section_screen();

    screen.handle_key_event(key(KeyCode::Tab));
    screen.handle_key_event(key(KeyCode::PageDown));

    assert_eq!(screen.clamped_arrangement_cursor(), 10);
    assert_eq!(screen.clamped_section_cursor(), 0);
}

/// 行が 10 未満でも溢れない（arrangement は 0 行にもできる）。
#[test]
fn the_page_keys_do_nothing_in_a_short_or_empty_pane() {
    let mut screen = three_section_screen();

    screen.handle_key_event(key(KeyCode::PageDown));
    assert_eq!(screen.clamped_section_cursor(), 2);

    let mut empty = ChordChartScreen::new(Song::empty());
    empty.handle_key_event(key(KeyCode::PageDown));
    empty.handle_key_event(key(KeyCode::PageUp));
    assert_eq!(empty.section_cursor, 0);
}

#[test]
fn j_and_k_move_the_cursor_of_the_focused_pane_only() {
    let mut screen = three_section_screen();

    screen.handle_key_event(key(KeyCode::Char('j')));
    assert_eq!(screen.clamped_section_cursor(), 1);
    assert_eq!(screen.clamped_arrangement_cursor(), 0);

    screen.handle_key_event(key(KeyCode::Tab));
    screen.handle_key_event(key(KeyCode::Char('j')));
    assert_eq!(screen.clamped_arrangement_cursor(), 1);
    assert_eq!(screen.clamped_section_cursor(), 1);

    screen.handle_key_event(key(KeyCode::Char('k')));
    assert_eq!(screen.clamped_arrangement_cursor(), 0);
}

#[test]
fn the_arrow_keys_do_the_same_as_j_and_k() {
    let mut screen = three_section_screen();

    screen.handle_key_event(key(KeyCode::Down));
    screen.handle_key_event(key(KeyCode::Down));
    assert_eq!(screen.clamped_section_cursor(), 2);

    screen.handle_key_event(key(KeyCode::Up));
    assert_eq!(screen.clamped_section_cursor(), 1);
}

/// 押しっぱなしで溢れさせたあと、`k` 1 回で必ず 1 行戻ること。
/// 丸めずに足し引きすると、ここで何十回も無反応になる。
#[test]
fn the_cursor_stops_at_the_ends_instead_of_running_away() {
    let mut screen = three_section_screen();

    for _ in 0..20 {
        screen.handle_key_event(key(KeyCode::Char('j')));
    }
    assert_eq!(screen.section_cursor, 2);

    screen.handle_key_event(key(KeyCode::Char('k')));
    assert_eq!(screen.section_cursor, 1);

    for _ in 0..20 {
        screen.handle_key_event(key(KeyCode::Char('k')));
    }
    assert_eq!(screen.section_cursor, 0);
}

/// 行が 0 のときにカーソルキーを押しても panic しない（arrangement は空にできる）。
#[test]
fn moving_the_cursor_in_an_empty_pane_does_nothing() {
    let mut screen = ChordChartScreen::new(Song::empty());

    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Tab));
    screen.handle_key_event(key(KeyCode::Char('j')));

    assert_eq!(screen.section_cursor, 0);
    assert_eq!(screen.arrangement_cursor, 0);
}
