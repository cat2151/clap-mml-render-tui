use super::*;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use cmrt_tui_core::buffer_test::find_text_ignoring_spaces;

fn key(code: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(code), KeyModifiers::NONE)
}

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// ヘッダ行の中身だけ。画面の外枠（`│`）は落とす。
fn header_text(screen: &ChordChartScreen) -> String {
    squeeze(&row_text(&render(screen), 1))
        .trim_matches('│')
        .to_string()
}

/// 入力欄へ `text` を打ち込んで `Enter` で確定する。既定値が入っているので先に消す。
fn retype_prefix(screen: &mut ChordChartScreen, text: &str) {
    screen.handle_key_event(key('b'));
    for _ in 0..80 {
        screen.handle_key_event(plain(KeyCode::Backspace));
    }
    for ch in text.chars() {
        screen.handle_key_event(key(ch));
    }
    screen.handle_key_event(plain(KeyCode::Enter));
}

/// 4.1 の完成イメージのヘッダがそのまま出るか。prefix は**解釈せず素通し**なので、
/// 画面に出る文字列は `Song::prefix` そのもの。
#[test]
fn the_header_shows_the_prefix_verbatim() {
    let screen = screen_with(song_of_eight_rows());

    let buffer = render(&screen);

    find_text_ignoring_spaces(&buffer, "Key=CBPM120");
}

/// ヘッダには小節数も所要時間も出さない（どちらも進行を解釈しないと出せない＝演奏スコープ）。
/// prefix 以外の文字（`Key:` のようなラベル）も足さない。
#[test]
fn the_header_shows_neither_a_measure_count_nor_a_duration() {
    let screen = screen_with(song_of_eight_rows());

    let header = header_text(&screen);

    assert!(!header.contains("小節"), "{header:?}");
    assert_eq!(header, "Key=CBPM120", "{header:?}");
}

/// 曲を変えたらヘッダも必ず変わる。
#[test]
fn the_header_follows_the_song() {
    let mut song = song_of_eight_rows();
    song.prefix = "Key=F# BPM60".to_string();

    let buffer = render(&screen_with(song));

    find_text_ignoring_spaces(&buffer, "Key=F#BPM60");
}

#[test]
fn the_screen_is_titled_chord_chart() {
    let buffer = render(&ChordChartScreen::default());

    find_text_ignoring_spaces(&buffer, "ChordChart");
}

/// `b` → 打つ → `Enter` で**画面の**ヘッダが変わる（状態だけの assert では取りこぼす）。
#[test]
fn b_then_enter_changes_the_header_on_the_screen() {
    let mut screen = screen_with(song_of_eight_rows());
    assert_eq!(header_text(&screen), "Key=CBPM120");

    retype_prefix(&mut screen, "Key=A BPM90");

    assert!(!screen.line_input_open(), "確定したら閉じること");
    assert_eq!(header_text(&screen), "Key=ABPM90");
}

/// `b` の入力は**検証しない**。chord2mml が読めない綴りでも空文字でもヘッダへ出る。
#[test]
fn the_prefix_input_accepts_anything_including_an_empty_string() {
    let mut screen = screen_with(song_of_eight_rows());

    retype_prefix(&mut screen, "BPM:0 なんでも");
    assert_eq!(header_text(&screen), "BPM:0なんでも");
    assert!(!screen.line_input_open(), "理由を出して開いたままにしない");

    retype_prefix(&mut screen, "");
    assert_eq!(header_text(&screen), "", "空文字も受ける");
}

/// `Esc` は打った内容を捨てる。ヘッダは元のまま。
#[test]
fn esc_throws_the_prefix_edit_away() {
    let mut screen = screen_with(song_of_eight_rows());

    screen.handle_key_event(key('b'));
    for ch in "zzz".chars() {
        screen.handle_key_event(key(ch));
    }
    screen.handle_key_event(plain(KeyCode::Esc));

    assert!(!screen.line_input_open());
    assert_eq!(header_text(&screen), "Key=CBPM120");
}

/// prefix をどう書こうと右 pane には時刻が出ない（開始時刻の列を消したこと）。
#[test]
fn the_arrangement_pane_shows_no_start_times_for_any_prefix() {
    let mut screen = screen_with(song_of_eight_rows());

    for prefix in ["Key=C BPM120", "Key=C BPM60"] {
        screen.song.prefix = prefix.to_string();
        let pane = pane_text(&render(&screen), Pane::Arrangement);
        assert!(
            !pane.contains(':'),
            "prefix {prefix} で時刻が出ている: {pane}"
        );
    }
}
