//! chord 行 → MML 変換の単体テスト。
//!
//! **期待値は推測で書かない。** すべて
//! `chord2mml-rust/target/debug/chord2mml.exe "<組み立てた入力>"` を実際に走らせて
//! 得た出力をそのまま置いてある（引き継ぎ資料 3.2 の表の、縦棒で囲んだ版）。

use super::{chord2mml_input, generate_mml_from_chord_cell};

#[test]
fn a_chord_cell_alone_is_wrapped_in_bars() {
    assert_eq!(chord2mml_input("", "", "I").as_deref(), Some("| I |"));
}

#[test]
fn the_chord_row_init_comes_before_the_track_directive() {
    assert_eq!(
        chord2mml_input("key:G", "close", "I-IV").as_deref(),
        Some("key:G close | I-IV |")
    );
}

#[test]
fn surrounding_whitespace_is_trimmed_out_of_every_part() {
    assert_eq!(
        chord2mml_input("  key:G  ", "  close  ", "  I-IV  ").as_deref(),
        Some("key:G close | I-IV |")
    );
}

#[test]
fn an_empty_chord_cell_has_nothing_to_convert() {
    assert_eq!(chord2mml_input("key:G", "close", "   "), None);
}

#[test]
fn a_single_degree_becomes_a_whole_note_triad() {
    assert_eq!(
        generate_mml_from_chord_cell("", "", "I"),
        "v11/*|*/'c1eg'/*|*/"
    );
}

#[test]
fn three_chords_are_split_evenly_inside_one_measure() {
    assert_eq!(
        generate_mml_from_chord_cell("", "", "I-IV-V"),
        "v11/*|*/'c3eg''f3a<c''g3b<d'/*|*/"
    );
}

#[test]
fn the_key_on_the_chord_row_init_transposes_the_whole_measure() {
    assert_eq!(
        generate_mml_from_chord_cell("key:G", "", "I-IV"),
        "v11/*|*/'g2b<d''<c2eg'/*|*/"
    );
}

#[test]
fn a_voicing_directive_from_the_track_init_reaches_chord2mml() {
    assert_eq!(
        generate_mml_from_chord_cell("", "close", "I-IV"),
        "v11/*|*/'c2eg''f2a<c'/*|*/"
    );
}

#[test]
fn the_key_on_the_track_init_overrides_the_key_on_the_chord_row_init() {
    // chord2mml の key: は後勝ち。連結順が「chord 行 init → track init」なので、
    // track init に書いた key が勝つ。
    let whole_song_in_c = generate_mml_from_chord_cell("key:C", "", "I-IV");
    let this_track_in_g = generate_mml_from_chord_cell("key:C", "key:G", "I-IV");
    assert_eq!(whole_song_in_c, "v11/*|*/'c2eg''f2a<c'/*|*/");
    assert_eq!(this_track_in_g, "v11/*|*/'g2b<d''<c2eg'/*|*/");
}

#[test]
fn an_empty_chord_cell_generates_no_mml() {
    assert_eq!(generate_mml_from_chord_cell("key:G", "close", ""), "");
}

#[test]
fn a_conversion_error_falls_back_to_silence_instead_of_breaking_playback() {
    // conductor の body（t120）を混ぜると chord2mml は必ず Syntax error になる。
    assert_eq!(generate_mml_from_chord_cell("t120", "", "I"), "");
    assert_eq!(generate_mml_from_chord_cell("", "", "???"), "");
}

#[test]
fn the_generated_mml_is_accepted_by_the_mml_parser() {
    // chord2mml の出力（/*|*/ 付き）がそのまま mmlabc-to-smf を通ることの確認。
    let mml = generate_mml_from_chord_cell("", "", "I-IV");
    assert!(mmlabc_to_smf::mml_to_smf_bytes(&mml).is_ok());
}

// --- split_progression_into_measures --------------------------------------
// grid は 1 セル = 1 小節なので、進行は 1 コードずつに切ってから配る。

use super::split_progression_into_measures as split;

#[test]
fn a_progression_is_split_into_one_chord_per_measure() {
    assert_eq!(split("I-IV-V-I"), vec!["I", "IV", "V", "I"]);
}

#[test]
fn a_single_chord_stays_a_single_measure() {
    assert_eq!(split("I"), vec!["I"]);
}

#[test]
fn a_key_directive_is_not_counted_as_a_measure() {
    // key: は小節を消費しない。コードだけが小節になる。
    assert_eq!(split("key:G I-IV"), vec!["I", "IV"]);
}

#[test]
fn an_unparsable_progression_yields_no_measures() {
    assert!(split("zzz").is_empty());
    assert!(split("").is_empty());
    assert!(split("   ").is_empty());
}

#[test]
fn a_chord_comes_back_in_the_normalized_spelling() {
    // パーサの正規形で返る。鳴る音は同じ（下の 2 つの MML が一致する）。
    assert_eq!(split("I-V-vi"), vec!["I", "V", "VIm"]);
    assert_eq!(
        generate_mml_from_chord_cell("", "close", "vi"),
        generate_mml_from_chord_cell("", "close", "VIm")
    );
}
