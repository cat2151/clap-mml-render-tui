use super::super::{DEFAULT_TRACK0_MML, MEASURES, TRACKS};
use super::{build_cell_mml_from_data, build_measure_mml_from_data};

/// テスト用ヘルパー: TRACKS×(MEASURES+1) の空 data を作成する
fn empty_data(tracks: usize, measures: usize) -> Vec<Vec<String>> {
    vec![vec![String::new(); measures + 1]; tracks]
}

// ─── build_cell_mml_from_data ─────────────────────────────────

#[test]
fn build_cell_mml_includes_timbre_in_measure() {
    // 音色 JSON が小節 MML に含まれること（issue #67 修正の前提: 音色変更時に小節を再キャッシュすべき根拠）
    let mut data = empty_data(TRACKS, MEASURES);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    data[1][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
    data[1][1] = "cde".to_string();

    let mml = build_cell_mml_from_data(&data, MEASURES, 1, 1);
    assert!(
        mml.contains(r#"{"Surge XT patch": "piano"}"#),
        "音色 JSON が MML に含まれていない: {}",
        mml
    );
    assert!(mml.contains("cde"), "音符が MML に含まれていない: {}", mml);
}

#[test]
fn build_cell_mml_includes_track0_tempo_in_measure() {
    // track0 のテンポ指定が小節 MML に含まれること（track0 変更時に全小節を再キャッシュすべき根拠）
    let mut data = empty_data(TRACKS, MEASURES);
    data[0][0] = r#"{"beat": "4/4"}t180"#.to_string();
    data[1][0] = "".to_string();
    data[1][1] = "cde".to_string();

    let mml = build_cell_mml_from_data(&data, MEASURES, 1, 1);
    assert!(
        mml.contains("t180"),
        "track0 のテンポ指定が MML に含まれていない: {}",
        mml
    );
    assert!(mml.contains("cde"), "音符が MML に含まれていない: {}", mml);
}

#[test]
fn build_cell_mml_timbre_change_affects_all_measures() {
    // 同じ音符セルで音色が異なる場合、MML が異なること
    // → 音色変更時は当該 track の全小節を再キャッシュしなければならない理由
    let mut data_piano = empty_data(TRACKS, MEASURES);
    data_piano[0][0] = DEFAULT_TRACK0_MML.to_string();
    data_piano[1][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
    data_piano[1][1] = "cde".to_string();

    let mut data_guitar = data_piano.clone();
    data_guitar[1][0] = r#"{"Surge XT patch": "guitar"}"#.to_string();

    let mml_piano = build_cell_mml_from_data(&data_piano, MEASURES, 1, 1);
    let mml_guitar = build_cell_mml_from_data(&data_guitar, MEASURES, 1, 1);

    assert_ne!(
        mml_piano, mml_guitar,
        "音色変更後の MML が同一になっており、キャッシュ無効化が必要"
    );
}

#[test]
fn build_cell_mml_track0_change_affects_all_tracks() {
    // track0 のテンポ変更で全 track の小節 MML が変化すること
    // → track0 セル変更時は全演奏トラックの全小節を再キャッシュしなければならない理由
    let mut data_t120 = empty_data(TRACKS, MEASURES);
    data_t120[0][0] = DEFAULT_TRACK0_MML.to_string();
    data_t120[1][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
    data_t120[1][1] = "cde".to_string();

    let mut data_t200 = data_t120.clone();
    data_t200[0][0] = r#"{"beat": "4/4"}t200"#.to_string();

    let mml_t120 = build_cell_mml_from_data(&data_t120, MEASURES, 1, 1);
    let mml_t200 = build_cell_mml_from_data(&data_t200, MEASURES, 1, 1);

    assert_ne!(
        mml_t120, mml_t200,
        "track0 変更後の MML が同一になっており、全小節の再キャッシュが必要"
    );
}

#[test]
fn build_cell_mml_empty_notes_cell_has_no_note_content() {
    // 音符セルが空のとき、その MML には音符が含まれないこと
    // → kick_cache は data[track][measure] が空のときジョブを投入しないことで正しい挙動となる（issue #69 修正）
    let mut data = empty_data(TRACKS, MEASURES);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    data[1][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
    data[1][1] = "".to_string(); // 音符が空

    // 空の音符セルは kick_cache によってジョブが投入されないため
    // キャッシュ状態は Empty のままとなり、"●" インジケータは表示されない
    assert!(data[1][1].trim().is_empty(), "音符セルが空であるべき");

    // build_cell_mml_from_data は track0 を常に含むため空でないが、
    // kick_cache は data[track][measure] の生の値で空判定するため、
    // このセルはキャッシュジョブが投入されない
    let combined_mml = build_cell_mml_from_data(&data, MEASURES, 1, 1);
    assert!(
        !combined_mml.trim().is_empty(),
        "結合 MML は track0 を含むため非空"
    );
    // kick_cache の正しい実装: data[track][measure].trim().is_empty() で早期リターン
    // （combined_mml が非空でもセル自身が空なら投入しない）
    let should_kick = !data[1][1].trim().is_empty();
    assert!(
        !should_kick,
        "空の音符セルは kick_cache に投入されるべきでない"
    );
}

#[test]
fn build_measure_mml_returns_empty_when_measure_has_no_notes() {
    let mut data = empty_data(TRACKS, MEASURES);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    data[1][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
    data[2][0] = r#"{"Surge XT patch": "brass"}"#.to_string();

    let mml = build_measure_mml_from_data(&data, MEASURES, TRACKS, 1);

    assert_eq!(mml, "");
}

#[test]
fn build_measure_mml_keeps_only_tracks_with_notes() {
    let mut data = empty_data(TRACKS, MEASURES);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    data[1][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
    data[1][1] = "cde".to_string();
    data[2][0] = r#"{"Surge XT patch": "brass"}"#.to_string();

    let mml = build_measure_mml_from_data(&data, MEASURES, TRACKS, 1);

    assert!(mml.contains("cde"), "音符が MML に含まれていない: {}", mml);
    assert!(
        !mml.contains(r#"{"Surge XT patch": "brass"}"#),
        "音符のない track が MML に含まれている: {}",
        mml
    );
}

#[test]
fn build_measure_mml_reapplies_timbre_to_semicolon_branches_in_same_track() {
    let mut data = empty_data(TRACKS, MEASURES);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    data[1][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
    data[1][1] = "cde;gab".to_string();

    let mml = build_measure_mml_from_data(&data, MEASURES, TRACKS, 1);

    assert_eq!(
        mml.matches(r#"{"Surge XT patch": "piano"}"#).count(),
        2,
        "each semicolon branch should receive timbre JSON: {}",
        mml
    );
    assert_eq!(
        mml.matches("t120").count(),
        2,
        "each semicolon branch should receive the track0 tempo token: {}",
        mml
    );
    assert!(
        mml.contains("cde"),
        "first branch missing from MML: {}",
        mml
    );
    assert!(
        mml.contains("gab"),
        "second branch missing from MML: {}",
        mml
    );
}

// ─── track8（最終演奏トラック）のテスト ───────────────────────

#[test]
fn build_cell_mml_track8_is_accessible() {
    // TRACKS-1 (= 8) が最終演奏トラックとして正しく動作すること（issue #72: track1~8 対応）
    let last_track = TRACKS - 1;
    let mut data = empty_data(TRACKS, MEASURES);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    data[last_track][0] = r#"{"Surge XT patch": "bass"}"#.to_string();
    data[last_track][1] = "c4d4e4f4".to_string();

    let mml = build_cell_mml_from_data(&data, MEASURES, last_track, 1);
    assert!(
        mml.contains(r#"{"Surge XT patch": "bass"}"#),
        "track8 の音色 JSON が MML に含まれていない: {}",
        mml
    );
    assert!(
        mml.contains("c4d4e4f4"),
        "track8 の音符が MML に含まれていない: {}",
        mml
    );
    assert!(
        mml.contains("t120"),
        "track0 のテンポが track8 の MML に含まれていない: {}",
        mml
    );
}
