//! chord 行の保存・読み込み。
//!
//! chord 行は `tracks` ではなく専用フィールドへ置き、
//! 中身が空なら書き出さない（`tracks` モジュールのコメント参照）。

use super::*;

/// chord 行を使っていない既存セーブは、読んで保存し直しても JSON が 1 バイトも変わらない。
///
/// これが崩れると、chord 行を触っていないユーザーのセーブが起動しただけで書き換わる。
#[test]
fn a_save_file_without_a_chord_row_is_rewritten_byte_for_byte() {
    let saved_json = serde_json::to_string_pretty(
        &serde_json::from_str::<DawSaveFile>(
            r#"{"tracks":[
                 {"track":0,"description":"tempo track","meas":[
                   {"meas":0,"description":"initial","mml":"{\"beat\":\"4/4\"}t120"}]},
                 {"track":1,"volume_db":-6,"meas":[
                   {"meas":0,"description":"initial","mml":"{\"Surge XT patch\": \"piano\"}"},
                   {"meas":1,"mml":"cde"}]},
                 {"track":2,"meas":[{"meas":2,"mml":"gab"}]}
               ]}"#,
        )
        .unwrap(),
    )
    .unwrap();

    let loaded: DawSaveFile = serde_json::from_str(&saved_json).unwrap();
    let (tracks, measures) = required_grid_size(&loaded);
    let mut data = empty_data(tracks, measures);
    let mut track_volumes = empty_track_volumes(tracks);
    apply_save_file_to_data(&loaded, &mut data, tracks, measures);
    apply_save_file_to_track_volumes(&loaded, &mut track_volumes, tracks);

    let rewritten =
        serde_json::to_string_pretty(&data_to_save_file(&data, &track_volumes, tracks, measures))
            .unwrap();

    assert_eq!(
        rewritten, saved_json,
        "chord 行を触っていないのに保存内容が変わっている"
    );
    assert!(
        !rewritten.contains("chord_track"),
        "空の chord 行を書き出している: {rewritten}"
    );
}

/// chord 行へ書いた内容は保存され、読み直しても同じ行へ戻る。
///
/// このとき演奏 track の保存番号は chord 行のぶんずれない（画面の `T1` = `"track": 1`）。
#[test]
fn a_chord_row_survives_the_save_and_load_roundtrip() {
    let mut data = empty_data(TRACKS, MEASURES);
    let track_volumes = empty_track_volumes(TRACKS);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    data[crate::CHORD_TRACK][0] = "key:G".to_string();
    data[crate::CHORD_TRACK][1] = "I-IV-V-I".to_string();
    data[crate::FIRST_PLAYABLE_TRACK][1] = "cde".to_string();

    let json =
        serde_json::to_string_pretty(&data_to_save_file(&data, &track_volumes, TRACKS, MEASURES))
            .unwrap();
    let loaded: DawSaveFile = serde_json::from_str(&json).unwrap();
    let mut restored = empty_data(TRACKS, MEASURES);
    apply_save_file_to_data(&loaded, &mut restored, TRACKS, MEASURES);

    assert_eq!(restored[crate::CHORD_TRACK][0], "key:G");
    assert_eq!(restored[crate::CHORD_TRACK][1], "I-IV-V-I");
    assert_eq!(restored[crate::FIRST_PLAYABLE_TRACK][1], "cde");
    assert!(
        json.contains(r#""track": 1"#),
        "演奏 track の保存番号が chord 行のぶんずれている: {json}"
    );
}

/// chord 行の中身だけで grid を広げられる（chord 行しか書いていないセーブも欠けない）。
#[test]
fn required_grid_size_counts_the_chord_row_measures() {
    let file: DawSaveFile =
        serde_json::from_str(r#"{"tracks":[],"chord_track":{"meas":[{"meas":7,"mml":"I-IV"}]}}"#)
            .unwrap();

    assert_eq!(required_grid_size(&file).1, 7);
}
