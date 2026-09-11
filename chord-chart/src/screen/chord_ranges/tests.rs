//! chord の範囲の写しの読み書き。**切る側（glue）は app のテストが見る**。
//!
//! ここで見るのは「写しを id で引くこと」「答えが無いときに 1 個へ倒れること」
//! 「カーソルが指している範囲を引けること」。
//!
//! 写しの中身はここでは**テスト用に組み立てる**（本物は glue が
//! `cmrt_chord::chord_source_ranges()` で切る。この crate は degrees を読まない）。

use std::ops::Range;

use crate::{ChordChartScreen, Pane, SectionId, Song};

/// テスト用の写し。`-` で割った各片の**バイト範囲**を返す。
///
/// 本物（`chord_source_ranges`）と切れ方が同じになる綴りだけをテストに使うこと。
/// ここで欲しいのは「範囲がいくつ・どこか」であって、chord2mml の文法ではない。
fn ranges_of(degrees: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for part in degrees.split('-') {
        ranges.push(start..start + part.len());
        start += part.len() + '-'.len_utf8();
    }
    ranges
}

/// section 3 つ・arrangement 1 行（`Sabi` を指す）。左右で指す先が違うので、
/// pane ごとの引き先の違いが読める。
fn screen() -> ChordChartScreen {
    let mut song = Song::empty();
    song.push_section("A", "I-V-VIm-IV");
    song.push_section("B", "IIm-V-I");
    let sabi = song.push_section("Sabi", "IV-V-IIIm-VIm-IIm-V-I-I");
    song.arrangement = vec![sabi];
    ChordChartScreen::new(song)
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

fn degrees_of(screen: &ChordChartScreen, name: &str) -> String {
    screen
        .song
        .sections
        .iter()
        .find(|section| section.name == name)
        .expect("その名前の section があること")
        .degrees
        .clone()
}

/// 曲の全 section ぶんの写しを、degrees から組み立てて書き戻す（glue の代役）。
fn write_back_all(screen: &mut ChordChartScreen) {
    let ranges: Vec<_> = screen
        .song
        .sections
        .iter()
        .map(|section| (section.id, ranges_of(&section.degrees)))
        .collect();
    screen.set_chord_ranges(ranges);
}

/// 書き戻した答えは、行の index ではなく **section の id** で引ける。
#[test]
fn the_written_back_ranges_are_read_by_section_id() {
    let mut screen = screen();
    let a = section_id(&screen, "A");
    let sabi = section_id(&screen, "Sabi");

    screen.set_chord_ranges([
        (a, ranges_of(&degrees_of(&screen, "A"))),
        (sabi, ranges_of(&degrees_of(&screen, "Sabi"))),
    ]);

    assert_eq!(screen.chord_count(a), Some(4), "数は写しの長さから引く");
    assert_eq!(screen.chord_count(sabi), Some(8));
    assert_eq!(
        screen.chord_count(section_id(&screen, "B")),
        None,
        "答えを書き戻していない section は `None`（画面は切らない）"
    );
    assert_eq!(screen.chord_ranges_len(), 2);
}

/// **写しが届く前でも画面は落ちない。** カーソル行の chord 数は 1 と答えるので、
/// chord カーソルは 0 番から動きようがなく、反転する範囲も無い。
#[test]
fn an_empty_copy_answers_one_chord_and_no_range() {
    let screen = screen();

    assert_eq!(screen.chord_ranges_len(), 0, "新品の画面に写しは無い");
    assert_eq!(screen.cursor_chord_count(), 1);
    assert_eq!(screen.cursor_chord_range(), None);

    // 行が 1 つも無い曲でも同じ（`preview_target` が `None` になる経路）。
    assert_eq!(ChordChartScreen::default().cursor_chord_count(), 1);
    assert_eq!(ChordChartScreen::default().cursor_chord_range(), None);
}

/// カーソル行の chord 数は、**カーソルが動けばその行のもの**になる。
#[test]
fn the_cursor_count_follows_the_cursor_row() {
    let mut screen = screen();
    write_back_all(&mut screen);

    assert_eq!(screen.cursor_chord_count(), 4, "左 pane の 0 行目 = A");
    screen.section_cursor = 1;
    assert_eq!(screen.cursor_chord_count(), 3);

    // 右 pane は**参照先** section（`Sabi`）の数。行そのものは進行を持たない。
    screen.focus = Pane::Arrangement;
    assert_eq!(screen.cursor_chord_count(), 8);
}

/// カーソルが指している範囲で degrees を切ると、**その chord の綴り**になる。
/// 右 pane では参照先 section の綴り。
#[test]
fn the_cursor_range_cuts_the_chord_it_points_at() {
    let mut screen = screen();
    write_back_all(&mut screen);
    let a = degrees_of(&screen, "A");

    let cut = |screen: &ChordChartScreen, degrees: &str| {
        degrees[screen.cursor_chord_range().expect("範囲が引けること")].to_string()
    };

    assert_eq!(cut(&screen, &a), "I");
    screen.chord_cursor = 2;
    assert_eq!(cut(&screen, &a), "VIm");

    screen.focus = Pane::Arrangement;
    screen.chord_cursor = 2;
    let sabi = degrees_of(&screen, "Sabi");
    assert_eq!(cut(&screen, &sabi), "IIIm");
}

/// 読めない degrees は glue が 0 件を書き戻す。**写しには 0 件のまま持ち**、
/// カーソル行の数としては 1 個へ倒す（＝行全体を鳴らす側へ落ちる）。範囲は無い。
#[test]
fn a_zero_range_row_is_treated_as_a_single_chord() {
    let mut screen = screen();
    let a = section_id(&screen, "A");

    screen.set_chord_ranges([(a, Vec::new())]);

    assert_eq!(
        screen.chord_count(a),
        Some(0),
        "切った結果の 0 件は 0 のまま"
    );
    assert_eq!(
        screen.cursor_chord_count(),
        1,
        "chord カーソルの範囲としては 1 個扱い"
    );
    assert_eq!(screen.cursor_chord_range(), None, "反転する範囲は無い");
}

/// 古い写しが残っていても、**別の section の答えを返すことはない**。
///
/// index で持つとここが壊れる（並べ替え / 削除で行がずれる）。id で持つ限り、
/// 消えた section は `None`、動いた section は自分の答えのままになる。
#[test]
fn a_stale_copy_never_reports_another_sections_answer() {
    let mut screen = screen();
    let a = section_id(&screen, "A");
    write_back_all(&mut screen);

    // 写しを書き戻さずに曲だけを変える（`dd` と `Alt+↑` の結果と同じ形）。
    screen.song.sections.remove(0);
    screen.song.sections.swap(0, 1);

    assert_eq!(
        screen.chord_count(a),
        Some(4),
        "消した section の答えは残る"
    );
    assert_eq!(
        screen.cursor_chord_count(),
        8,
        "0 行目は `Sabi` になったので、答えも `Sabi` のもの"
    );
    screen.section_cursor = 1;
    assert_eq!(screen.cursor_chord_count(), 3);
}
