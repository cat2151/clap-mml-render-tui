//! chord 行から演奏 track の MML を生成する経路のテスト。
//!
//! 期待値はすべて、`Cargo.lock` と同じ revision のローカル `chord2mml.exe` に
//! **実際に同じ入力を通して得た出力**をそのまま書いている（推測で書かない）。
//!
//! ```text
//! $ chord2mml.exe "close | I-IV |"
//! v11/*|*/'c2eg''f2a<c'/*|*/
//! ```

use crate::mml::{
    build_cell_mml_from_data, build_measure_mml_from_data, cell_has_content,
    cell_is_generated_from_chord_row,
};
use crate::{CHORD_TRACK, DEFAULT_TRACK0_MML, FIRST_PLAYABLE_TRACK, MEASURES, TRACKS};
use mmlabc_to_smf::mml_preprocessor;
use serde_json::Value;

pub(super) const GENERATED_TRACK: usize = FIRST_PLAYABLE_TRACK;
pub(super) const OTHER_TRACK: usize = FIRST_PLAYABLE_TRACK + 1;

/// conductor と chord 行だけを埋めた data を作る。
pub(super) fn data_with_chord_row(
    chord_init: &str,
    chord_cells: &[(usize, &str)],
) -> Vec<Vec<String>> {
    let mut data = vec![vec![String::new(); MEASURES + 1]; TRACKS];
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    data[CHORD_TRACK][0] = chord_init.to_string();
    for &(measure, cell) in chord_cells {
        data[CHORD_TRACK][measure] = cell.to_string();
    }
    data
}

pub(super) fn generate_init(directive: &str) -> String {
    format!(r#"{{"generate from chord track": "{directive}"}}"#)
}

fn split_final_mml(mml: &str) -> (Option<Value>, String) {
    let preprocessed = mml_preprocessor::extract_embedded_json(mml);
    let json = preprocessed
        .embedded_json
        .as_deref()
        .map(|json| serde_json::from_str::<Value>(json).unwrap());
    (json, preprocessed.remaining_mml)
}

fn body_of(mml: &str) -> String {
    split_final_mml(mml).1
}

fn no_solo(tracks: usize) -> Vec<bool> {
    vec![false; tracks]
}

#[test]
fn a_generated_track_plays_the_chord_row_instead_of_its_empty_cell() {
    let mut data = data_with_chord_row("", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] = generate_init("close");

    let mml = build_cell_mml_from_data(&data, MEASURES, GENERATED_TRACK, 1);

    // chord2mml.exe "close | I-IV |" の出力そのまま。t120 は conductor の body。
    assert_eq!(body_of(&mml), "t120v11/*|*/'c2eg''f2a<c'/*|*/");
}

#[test]
fn a_track_without_the_generate_key_stays_empty() {
    let mut data = data_with_chord_row("", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] = r#"{"Surge XT patch": "piano"}"#.to_string();

    let mml = build_cell_mml_from_data(&data, MEASURES, GENERATED_TRACK, 1);

    assert_eq!(body_of(&mml), "t120");
    assert!(!cell_has_content(&data, GENERATED_TRACK, 1));
}

#[test]
fn a_handwritten_cell_wins_over_the_chord_row() {
    let mut data = data_with_chord_row("", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] = generate_init("close");
    data[GENERATED_TRACK][1] = "cde".to_string();

    let mml = build_cell_mml_from_data(&data, MEASURES, GENERATED_TRACK, 1);

    assert_eq!(body_of(&mml), "t120cde");
}

#[test]
fn an_empty_chord_row_cell_generates_nothing() {
    let mut data = data_with_chord_row("", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] = generate_init("close");

    // measure 2 の chord 行は空。
    let mml = build_cell_mml_from_data(&data, MEASURES, GENERATED_TRACK, 2);

    assert_eq!(body_of(&mml), "t120");
    assert!(!cell_has_content(&data, GENERATED_TRACK, 2));
}

#[test]
fn the_generate_key_alone_is_enough_even_with_an_empty_value() {
    let mut data = data_with_chord_row("", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] = generate_init("");

    // chord2mml.exe "| I-IV |" の出力そのまま。
    assert_eq!(
        body_of(&build_cell_mml_from_data(
            &data,
            MEASURES,
            GENERATED_TRACK,
            1
        )),
        "t120v11/*|*/'c2eg''f2a<c'/*|*/"
    );
}

#[test]
fn the_chord_row_init_and_the_track_directive_both_reach_chord2mml() {
    let mut data = data_with_chord_row("key:G", &[(1, "I")]);
    data[GENERATED_TRACK][0] = generate_init("close");

    // chord2mml.exe "key:G close | I |" の出力そのまま。
    assert_eq!(
        body_of(&build_cell_mml_from_data(
            &data,
            MEASURES,
            GENERATED_TRACK,
            1
        )),
        "t120v11/*|*/'g1b<d'/*|*/"
    );
}

#[test]
fn the_key_in_the_track_directive_overrides_the_key_on_the_chord_row() {
    let mut data = data_with_chord_row("key:C", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] = generate_init("key:G");

    // chord2mml.exe "key:C key:G | I-IV |" の出力そのまま（key は後勝ち）。
    assert_eq!(
        body_of(&build_cell_mml_from_data(
            &data,
            MEASURES,
            GENERATED_TRACK,
            1
        )),
        "t120v11/*|*/'g2b<d''<c2eg'/*|*/"
    );
}

#[test]
fn the_init_column_itself_is_never_generated() {
    let mut data = data_with_chord_row("", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] = generate_init("close");

    // measure 0（init 列）は音符列ではないので生成しない。
    let mml = build_cell_mml_from_data(&data, MEASURES, GENERATED_TRACK, 0);

    assert_eq!(body_of(&mml), "t120");
}

#[test]
fn the_chord_row_itself_is_never_generated_from_itself() {
    let data = data_with_chord_row("key:C", &[(1, "I-IV")]);

    assert!(!cell_is_generated_from_chord_row(&data, CHORD_TRACK, 1));
}

#[test]
fn the_generate_key_survives_into_the_json_prefix_and_the_mml_still_parses() {
    let mut data = data_with_chord_row("", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] =
        r#"{"Surge XT patch": "piano", "generate from chord track": "close"}"#.to_string();

    let mml = build_cell_mml_from_data(&data, MEASURES, GENERATED_TRACK, 1);

    let json = split_final_mml(&mml).0.expect("JSON prefix");
    assert_eq!(json["Surge XT patch"], "piano");
    assert_eq!(json["generate from chord track"], "close");
    // 未知のキーが混ざっても MML パーサは受け付ける（実測済み）。
    assert!(mmlabc_to_smf::mml_to_smf_bytes(&mml).is_ok());
}

#[test]
fn the_measure_mml_includes_the_generated_track() {
    let mut data = data_with_chord_row("", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] = generate_init("close");
    data[OTHER_TRACK][1] = "cde".to_string();

    let mml = build_measure_mml_from_data(&data, MEASURES, TRACKS, 1, &no_solo(TRACKS));

    assert_eq!(body_of(&mml), "t120v11/*|*/'c2eg''f2a<c'/*|*/;t120cde");
}

#[test]
fn the_measure_mml_leaves_out_a_generated_track_whose_chord_cell_is_empty() {
    let mut data = data_with_chord_row("", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] = generate_init("close");
    data[OTHER_TRACK][2] = "cde".to_string();

    let mml = build_measure_mml_from_data(&data, MEASURES, TRACKS, 2, &no_solo(TRACKS));

    assert_eq!(body_of(&mml), "t120cde");
}

#[test]
fn changing_a_chord_row_cell_changes_the_cache_hash_of_the_generated_track() {
    use cmrt_history::daw_cache_mml_hash;

    let mut data = data_with_chord_row("", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] = generate_init("close");
    let before = daw_cache_mml_hash(&build_cell_mml_from_data(
        &data,
        MEASURES,
        GENERATED_TRACK,
        1,
    ));

    data[CHORD_TRACK][1] = "I-IV-V-I".to_string();
    let after = daw_cache_mml_hash(&build_cell_mml_from_data(
        &data,
        MEASURES,
        GENERATED_TRACK,
        1,
    ));

    assert_ne!(before, after);
}

#[test]
fn changing_the_chord_row_init_changes_the_cache_hash_of_the_generated_track() {
    use cmrt_history::daw_cache_mml_hash;

    let mut data = data_with_chord_row("", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] = generate_init("close");
    let before = daw_cache_mml_hash(&build_cell_mml_from_data(
        &data,
        MEASURES,
        GENERATED_TRACK,
        1,
    ));

    data[CHORD_TRACK][0] = "key:G".to_string();
    let after = daw_cache_mml_hash(&build_cell_mml_from_data(
        &data,
        MEASURES,
        GENERATED_TRACK,
        1,
    ));

    assert_ne!(before, after);
}

#[test]
fn changing_the_track_directive_changes_the_cache_hash() {
    use cmrt_history::daw_cache_mml_hash;

    let mut data = data_with_chord_row("", &[(1, "I-IV-V-I")]);
    data[GENERATED_TRACK][0] = generate_init("");
    let before = daw_cache_mml_hash(&build_cell_mml_from_data(
        &data,
        MEASURES,
        GENERATED_TRACK,
        1,
    ));

    data[GENERATED_TRACK][0] = generate_init("octave down");
    let after = daw_cache_mml_hash(&build_cell_mml_from_data(
        &data,
        MEASURES,
        GENERATED_TRACK,
        1,
    ));

    assert_ne!(before, after);
    // chord2mml.exe "octave down | I-IV-V-I |" の出力そのまま。
    assert_eq!(
        body_of(&build_cell_mml_from_data(
            &data,
            MEASURES,
            GENERATED_TRACK,
            1
        )),
        "t120v11/*|*/'>c4eg''>f4a<c''>g4b<d''>c4eg'/*|*/"
    );
}

#[test]
fn a_generated_cell_counts_as_having_content() {
    let mut data = data_with_chord_row("", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] = generate_init("close");

    assert!(cell_has_content(&data, GENERATED_TRACK, 1));
    assert!(cell_is_generated_from_chord_row(&data, GENERATED_TRACK, 1));
    // 手書きがあるセルは「生成されるセル」ではない。
    data[GENERATED_TRACK][2] = "cde".to_string();
    assert!(cell_has_content(&data, GENERATED_TRACK, 2));
    assert!(!cell_is_generated_from_chord_row(&data, GENERATED_TRACK, 2));
}

#[test]
fn a_broken_chord_cell_falls_back_to_silence_instead_of_breaking_the_measure() {
    let mut data = data_with_chord_row("", &[(1, "???")]);
    data[GENERATED_TRACK][0] = generate_init("close");
    data[OTHER_TRACK][1] = "cde".to_string();

    assert!(!cell_has_content(&data, GENERATED_TRACK, 1));
    assert_eq!(
        body_of(&build_measure_mml_from_data(
            &data,
            MEASURES,
            TRACKS,
            1,
            &no_solo(TRACKS)
        )),
        "t120cde"
    );
}
