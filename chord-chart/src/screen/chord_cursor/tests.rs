//! chord カーソルのキー（`h` `l` `←` `→` と `Tab`）。
//!
//! 見るのは**押した結果として立つ要求**。音は出さない（鳴らすのは app 側）。
//! chord の範囲の写しは glue が書き戻すものなので、ここでは
//! [`ChordChartScreen::set_chord_ranges`] で直接与える（数はその長さ）。

use super::*;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{ChordChartAction, PreviewRequest, PreviewVoicingContext, SectionId, Song};

/// chord が `count` 個ある行ぶんの、テスト用の写し。
///
/// **中身の位置はここでは意味を持たない**（このモジュールが見るのは番号だけ。
/// 範囲そのものを見るのは `chord_ranges` と `ui` のテスト）。
fn ranges(count: usize) -> Vec<std::ops::Range<usize>> {
    (0..count).map(|index| index..index + 1).collect()
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// section 3 つ（chord 数 4 / 3 / 2）・arrangement 2 行（`Sabi` → `A`）。
///
/// 行ごとに chord 数を変えてあるので、繰り上がりの「末尾」が行によって違うことが読める。
/// 右 pane の参照先は左 pane の行番号とずらしてある（1 行目 = `Sabi`）。
fn screen() -> ChordChartScreen {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    song.push_section("B", "IIm-V-I");
    let sabi = song.push_section("Sabi", "IV-V");
    song.arrangement = vec![sabi, a];
    let mut screen = ChordChartScreen::new(song);
    let counts: Vec<_> = [("A", 4), ("B", 3), ("Sabi", 2)]
        .iter()
        .map(|(name, count)| (section_id(&screen, name), ranges(*count)))
        .collect();
    screen.set_chord_ranges(counts);
    // 画面へ入った直後の 1 回を捨てて、キーの結果だけを見られるようにする。
    screen.enter();
    screen.take_preview();
    screen
}

fn section_id(screen: &ChordChartScreen, name: &str) -> SectionId {
    screen
        .song
        .sections
        .iter()
        .find(|section| section.name == name)
        .expect("その名前の section があること")
        .id
}

fn taken(screen: &mut ChordChartScreen) -> PreviewRequest {
    screen.take_preview().expect("preview 要求が立つこと")
}

fn arrangement_request(
    name: &str,
    degrees: &str,
    selected: usize,
    chord_index: Option<usize>,
) -> PreviewRequest {
    PreviewRequest {
        name: name.to_string(),
        degrees: degrees.to_string(),
        chord_index,
        voicing_context: PreviewVoicingContext {
            progressions: vec!["IV-V".to_string(), "I-V-VIm-IV".to_string()],
            selected,
        },
    }
}

/// `l` で次の chord、`h` で戻る。鳴らすのは**その chord 1 つ**（`chord_index` が埋まる）。
#[test]
fn l_and_h_step_through_the_chords_of_the_row() {
    let mut screen = screen();

    screen.handle_key_event(key(KeyCode::Char('l')));

    assert_eq!(screen.chord_cursor(), 1);
    assert_eq!(
        taken(&mut screen),
        PreviewRequest::section("A", "I-V-VIm-IV", Some(1))
    );

    screen.handle_key_event(key(KeyCode::Char('h')));

    assert_eq!(screen.chord_cursor(), 0);
    assert_eq!(taken(&mut screen).chord_index, Some(0));
    assert_eq!(screen.clamped_section_cursor(), 0, "行は動かない");
}

/// `←` `→` は `h` `l` と同じキー。片方だけ配線を忘れても気づけるように両方押す。
#[test]
fn the_arrow_keys_step_through_the_chords_too() {
    let mut screen = screen();

    screen.handle_key_event(key(KeyCode::Right));
    screen.handle_key_event(key(KeyCode::Right));
    assert_eq!(taken(&mut screen).chord_index, Some(2));

    screen.handle_key_event(key(KeyCode::Left));
    assert_eq!(taken(&mut screen).chord_index, Some(1));
}

/// 行末で `l` → **次の行の先頭 chord**。行が変わったことも要求の名前で分かる。
#[test]
fn l_at_the_end_of_a_row_carries_over_to_the_next_row() {
    let mut screen = screen();
    for _ in 0..3 {
        screen.handle_key_event(key(KeyCode::Char('l')));
    }
    assert_eq!(taken(&mut screen).chord_index, Some(3), "A の末尾");

    screen.handle_key_event(key(KeyCode::Char('l')));

    assert_eq!(screen.clamped_section_cursor(), 1);
    assert_eq!(
        taken(&mut screen),
        PreviewRequest::section("B", "IIm-V-I", Some(0))
    );
}

/// 行頭で `h` → **前の行の末尾 chord**。末尾の番号は移った先の行の chord 数で決まる
/// （2 chord の行から 3 chord の行へ戻れば 2 番）。
#[test]
fn h_at_the_start_of_a_row_carries_over_to_the_end_of_the_previous_row() {
    let mut screen = screen();
    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.take_preview();
    assert_eq!(screen.clamped_section_cursor(), 2, "Sabi（2 chord）");

    screen.handle_key_event(key(KeyCode::Char('h')));

    assert_eq!(screen.clamped_section_cursor(), 1);
    assert_eq!(
        taken(&mut screen),
        PreviewRequest::section("B", "IIm-V-I", Some(2)),
        "3 chord の行の末尾は 2 番"
    );
}

/// 最終行の末尾で `l` はそこで止まる。**要求も立たない**（連打で鳴り直さない）。
#[test]
fn l_at_the_very_end_asks_for_nothing() {
    let mut screen = screen();
    screen.handle_key_event(key(KeyCode::PageDown));
    screen.take_preview();
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.take_preview();
    assert_eq!(screen.chord_cursor(), 1, "Sabi の末尾");

    screen.handle_key_event(key(KeyCode::Char('l')));

    assert_eq!(screen.take_preview(), None);
    assert_eq!(screen.clamped_section_cursor(), 2);
    assert_eq!(screen.chord_cursor(), 1);
}

/// 先頭行の先頭で `h` も同じ（pane 移動もしない）。
#[test]
fn h_at_the_very_start_asks_for_nothing_and_stays_in_the_pane() {
    let mut screen = screen();

    screen.handle_key_event(key(KeyCode::Char('h')));

    assert_eq!(screen.take_preview(), None);
    assert_eq!(screen.focus, Pane::Sections);
    assert_eq!(screen.chord_cursor(), 0);
}

/// 繰り上がりは **pane をまたがない**。右 pane の最終行の末尾で `l` を押しても、
/// 左 pane へは戻らない。
#[test]
fn the_carry_over_never_crosses_to_the_other_pane() {
    let mut screen = screen();
    screen.handle_key_event(key(KeyCode::Tab));
    screen.take_preview();
    for _ in 0..10 {
        screen.handle_key_event(key(KeyCode::Char('l')));
        screen.take_preview();
    }

    assert_eq!(screen.focus, Pane::Arrangement);
    assert_eq!(screen.clamped_arrangement_cursor(), 1, "右 pane の最終行");
    assert_eq!(screen.chord_cursor(), 3, "参照先 `A` の末尾");
    assert_eq!(screen.take_preview(), None);
}

/// `Tab` は pane のトグル。`h` `l` は pane を動かさない。
#[test]
fn tab_toggles_the_pane_and_hl_does_not() {
    let mut screen = screen();

    screen.handle_key_event(key(KeyCode::Tab));
    assert_eq!(screen.focus, Pane::Arrangement);
    // 右 pane の 1 行目は参照先の `Sabi`（行番号ではない）。行全体を鳴らす。
    assert_eq!(
        taken(&mut screen),
        arrangement_request("Sabi", "IV-V", 0, None)
    );

    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.take_preview();
    screen.handle_key_event(key(KeyCode::Char('h')));
    screen.take_preview();
    assert_eq!(
        screen.focus,
        Pane::Arrangement,
        "`h` `l` は pane を動かさない"
    );

    screen.handle_key_event(key(KeyCode::Tab));
    assert_eq!(screen.focus, Pane::Sections);
}

/// `Shift+Tab`（`BackTab`）は割り当てない。押しても何も起きない。
#[test]
fn shift_tab_is_not_bound() {
    let mut screen = screen();

    screen.handle_key_event(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));

    assert_eq!(screen.focus, Pane::Sections);
    assert_eq!(screen.take_preview(), None);
}

/// `Tab` で pane を移ると chord カーソルは先頭へ戻る。
#[test]
fn switching_panes_resets_the_chord_cursor() {
    let mut screen = screen();
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.take_preview();

    screen.handle_key_event(key(KeyCode::Tab));

    assert_eq!(screen.chord_cursor(), 0);
    assert_eq!(taken(&mut screen).chord_index, None);
}

/// 行を移る `j` `PgDn` は chord カーソルを先頭へ戻し、**行全体**を鳴らす。
#[test]
fn moving_a_row_resets_the_chord_cursor_and_plays_the_whole_row() {
    for code in [KeyCode::Char('j'), KeyCode::PageDown] {
        let mut screen = screen();
        screen.handle_key_event(key(KeyCode::Char('l')));
        screen.handle_key_event(key(KeyCode::Char('l')));
        screen.take_preview();
        assert_eq!(screen.chord_cursor(), 2);

        screen.handle_key_event(key(code));

        assert_eq!(screen.chord_cursor(), 0, "{code:?} でリセットされること");
        assert_eq!(taken(&mut screen).chord_index, None, "{code:?}");
    }
}

/// 端で止まって**行が変わらなかった**ときは、chord カーソルを動かさない
/// （動かすと「押しても行は動かないのに聴いている chord だけ変わる」1 回になる）。
#[test]
fn a_row_key_that_did_not_move_keeps_the_chord_cursor() {
    let mut screen = screen();
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.take_preview();

    screen.handle_key_event(key(KeyCode::Char('k')));

    assert_eq!(screen.chord_cursor(), 1);
    assert_eq!(screen.take_preview(), None);
}

/// `Alt+↑` `Alt+↓`（行の並べ替え）でも先頭へ戻る。**preview は立てない**（編集キー）。
#[test]
fn moving_a_row_with_alt_resets_the_chord_cursor_without_a_preview() {
    let mut screen = screen();
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.take_preview();

    assert_eq!(
        screen.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT)),
        ChordChartAction::SongChanged
    );

    assert_eq!(screen.chord_cursor(), 0);
    assert_eq!(screen.take_preview(), None);
}

/// 右 pane でも同じように動き、**参照先** section の chord を指す
/// （`Sabi` は 2 chord なので、2 回目の `l` で次の行へ繰り上がる）。
#[test]
fn the_arrangement_pane_steps_through_the_referenced_sections_chords() {
    let mut screen = screen();
    screen.handle_key_event(key(KeyCode::Tab));
    screen.take_preview();

    screen.handle_key_event(key(KeyCode::Char('l')));
    assert_eq!(
        taken(&mut screen),
        arrangement_request("Sabi", "IV-V", 0, Some(1))
    );

    screen.handle_key_event(key(KeyCode::Char('l')));
    assert_eq!(screen.clamped_arrangement_cursor(), 1);
    assert_eq!(
        taken(&mut screen),
        arrangement_request("A", "I-V-VIm-IV", 1, Some(0))
    );
}

/// 写しがまだ無い（app が書き戻す前）行では chord 数が 1 なので、`l` は
/// **行内では動かず**そのまま次の行へ繰り上がる。落ちないこと。
#[test]
fn a_row_without_a_written_back_count_behaves_as_a_single_chord() {
    let mut song = Song::empty();
    song.push_section("A", "I-V-VIm-IV");
    song.push_section("B", "IIm-V-I");
    let mut screen = ChordChartScreen::new(song);
    screen.enter();
    screen.take_preview();

    screen.handle_key_event(key(KeyCode::Char('l')));

    assert_eq!(
        screen.clamped_section_cursor(),
        1,
        "行内に 2 つ目が無いので繰り上がる"
    );
    assert_eq!(taken(&mut screen).chord_index, Some(0));
}

/// 行が 1 つも無い曲でも落ちない（`h` `l` は何も起こさない）。
#[test]
fn an_empty_song_does_not_panic() {
    let mut screen = ChordChartScreen::new(Song::empty());

    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('h')));
    screen.handle_key_event(key(KeyCode::Tab));

    assert_eq!(screen.chord_cursor(), 0);
}

/// degrees を打ち替えて chord が減っても、chord カーソルは行の外を指さない。
///
/// host 側で degrees が更新されたとき、その場で丸め直す機会が無くても、読むときに
/// 丸めるのがここの役目。
#[test]
fn shrinking_the_degrees_never_leaves_the_chord_cursor_outside_the_row() {
    let mut screen = screen();
    for _ in 0..3 {
        screen.handle_key_event(key(KeyCode::Char('l')));
    }
    screen.take_preview();
    assert_eq!(screen.chord_cursor(), 3);

    // host editor で 2 chord へ打ち替えたのと同じ状態（写しも glue が書き戻す）。
    let a = section_id(&screen, "A");
    screen.song.sections[0].degrees = "I-V".to_string();
    screen.set_chord_ranges([(a, ranges(2))]);

    assert_eq!(screen.chord_cursor(), 1, "末尾へ丸める");
    screen.handle_key_event(key(KeyCode::Char('h')));
    assert_eq!(taken(&mut screen).chord_index, Some(0));
}

/// 1 行入力欄（`n` / `b`）が開いている間は、`h` `l` `Tab` は**入力欄へ行く**。
#[test]
fn the_line_input_swallows_the_chord_keys() {
    let mut screen = screen();
    screen.handle_key_event(key(KeyCode::Char('n')));
    for _ in 0..64 {
        screen.handle_key_event(key(KeyCode::Backspace));
    }

    screen.handle_key_event(key(KeyCode::Char('h')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Tab));

    assert!(screen.line_input_open());
    assert_eq!(screen.focus, Pane::Sections, "Tab が pane を動かしていない");
    assert_eq!(screen.chord_cursor(), 0);
    assert_eq!(screen.take_preview(), None);
    assert!(
        screen
            .line_input()
            .expect("入力欄が開いていること")
            .value()
            .starts_with("hl"),
        "`h` `l` は文字として入る"
    );
}
