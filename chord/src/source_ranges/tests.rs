use super::*;

use crate::ChordProgressionCatalog;

/// 配布中の `chord-progressions.json` のスナップショット。`progression/tests.rs` と同じもの。
const CATALOG_JSON: &str = include_str!("../../testdata/chord-progressions.json");

/// 範囲で切った結果まで読む。**件数だけを見ると位置がずれていても通ってしまう**ので、
/// このテストモジュールの assert はすべてこの関数を通す。
fn sliced(line: &str) -> Vec<&str> {
    chord_source_ranges(line)
        .into_iter()
        .map(|range| &line[range])
        .collect()
}

#[test]
fn a_hyphenated_progression_is_cut_into_its_chords() {
    assert_eq!(sliced("I-V-VIm-IV"), ["I", "V", "VIm", "IV"]);
    assert_eq!(chord_source_ranges("I-V-VIm-IV"), [0..1, 2..3, 4..7, 8..10]);
}

/// 区切りはハイフンだけではない。空白区切りでも同じ切れ方をする。
#[test]
fn a_space_separated_progression_is_cut_the_same_way() {
    assert_eq!(sliced("I V VIm IV"), ["I", "V", "VIm", "IV"]);
}

/// Key トークンは chord ではないので範囲に入らない。`Key=C` の 6 バイトぶん
/// ずれていないことを、切った文字列で確かめる。
#[test]
fn a_key_token_is_not_a_chord() {
    assert_eq!(sliced("Key=C I-V"), ["I", "V"]);
    assert_eq!(chord_source_ranges("Key=C I-V"), [6..7, 8..9]);
    assert_eq!(sliced("Key:C I-V"), ["I", "V"]);
    assert_eq!(sliced("drop2 I-V"), ["I", "V"]);
}

/// 読めない文字列は空。`Err` にはしない（呼び出し側は「行全体で 1 つ」へ倒す）。
#[test]
fn an_unreadable_line_has_no_chords() {
    assert!(chord_source_ranges("@@@").is_empty());
    assert!(chord_source_ranges("!!!").is_empty());
    assert!(chord_source_ranges("Key=").is_empty());
}

#[test]
fn an_empty_line_has_no_chords() {
    assert!(chord_source_ranges("").is_empty());
    assert!(chord_source_ranges("   ").is_empty());
    assert!(chord_source_ranges("\t").is_empty());
}

/// 末尾の区切り文字は、直前の chord の範囲に**含まれて**返る（chord2mml の CST が
/// そう切る）。切った結果は `"I-"` のように区切りつきになるが、それも chord2mml が
/// 読める綴りなので、こちらで削り直さない（削るとこの crate が文法を持つことになる）。
#[test]
fn a_trailing_separator_stays_inside_the_last_chord() {
    assert_eq!(sliced("I-"), ["I-"]);
    assert_eq!(sliced("I-V-"), ["I", "V-"]);
    assert_eq!(sliced("I "), ["I"], "末尾の空白は入らない。ハイフンだけ");
    assert_eq!(
        sliced("I-V-VIm-IV"),
        ["I", "V", "VIm", "IV"],
        "末尾以外は付かない"
    );
}

/// 小節線が混ざっても、切った結果はコード表記だけになる（`|` は範囲に入らない）。
#[test]
fn a_bar_line_is_not_part_of_any_chord() {
    assert_eq!(sliced("I-V|VIm-IV"), ["I", "V", "VIm", "IV"]);
    assert_eq!(sliced("| I-V | VIm-IV |"), ["I", "V", "VIm", "IV"]);
}

/// 全角の臨時記号は 1 文字 3 バイト。バイト範囲がずれていると、切った文字列が
/// 途中で割れて `&str` のスライスが panic する（＝この assert が守るのはそこ）。
#[test]
fn full_width_accidentals_keep_their_bytes_together() {
    assert_eq!(sliced("C♯m7-F♯7-BM7"), ["C♯m7", "F♯7", "BM7"]);
    assert_eq!(
        sliced("Key:C Ⅰ-Ⅴ").len(),
        0,
        "全角ローマ数字は chord2mml が読まない"
    );
}

/// 実カタログ相当（同梱スナップショット・60 件）で、切った結果が
/// **degrees をハイフンで割ったもの**と完全に一致すること。
///
/// 60 件すべてが切れる（読めないものは 0 件）。
/// 実際に配布されているカタログ
/// （`%LOCALAPPDATA%\clap-mml-render-tui\chord-progressions\progressions.json`）も
/// この同梱スナップショットと同じ 60 件で、全角も `|` も 1 件も含まない。
#[test]
fn the_shipped_catalog_is_cut_into_exactly_its_degrees() {
    let catalog = ChordProgressionCatalog::from_json(CATALOG_JSON).unwrap();
    assert_eq!(catalog.len(), 60);
    for entry in catalog.entries() {
        let expected = entry.degrees.split('-').collect::<Vec<_>>();
        assert_eq!(sliced(&entry.degrees), expected, "進行 {}", entry.degrees);
        assert_eq!(
            chord_source_ranges(&entry.degrees).len(),
            entry.chord_count(),
            "進行 {}",
            entry.degrees
        );
    }
}

/// 書きかけの入力でも落ちない。chord chart は 1 文字打つたびにここを通る。
///
/// 「書きかけ＝読めない＝空」ではない。空になる前提を置かずに全部通すだけ。
#[test]
fn half_written_input_does_not_panic() {
    for line in [
        "I", "I-", "-I", "Key", "Key=", "Key=C", "I--V", "'", ";", "C♯",
    ] {
        let _ = sliced(line);
    }
}
