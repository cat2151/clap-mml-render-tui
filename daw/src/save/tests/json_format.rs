//! セッション JSON の保存形式（track/meas の並びと description・volume）。

use super::*;

#[test]
fn daw_save_json_roundtrip_default_data() {
    // デフォルト data（track0/meas0 のみ非空）が JSON 経由で復元されること
    let mut data = empty_data(TRACKS, MEASURES);
    let track_volumes = empty_track_volumes(TRACKS);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();

    let file = data_to_save_file(&data, &track_volumes, TRACKS, MEASURES);
    let json = serde_json::to_string_pretty(&file).unwrap();

    let loaded_file: DawSaveFile = serde_json::from_str(&json).unwrap();
    let mut restored = empty_data(TRACKS, MEASURES);
    apply_save_file_to_data(&loaded_file, &mut restored, TRACKS, MEASURES);

    assert_eq!(restored[0][0], DEFAULT_TRACK0_MML);
    // 他のセルは空のまま
    assert!(restored[0][1].is_empty());
    assert!(restored[1][0].is_empty());
}

#[test]
fn daw_save_json_skips_empty_tracks_and_meas() {
    // 空トラック・空小節は JSON に含まれないこと
    let mut data = empty_data(TRACKS, MEASURES);
    let track_volumes = empty_track_volumes(TRACKS);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    data[crate::FIRST_PLAYABLE_TRACK][1] = "cde".to_string();
    // track2..8, meas2..MEASURES は空

    let file = data_to_save_file(&data, &track_volumes, TRACKS, MEASURES);
    let json = serde_json::to_string_pretty(&file).unwrap();

    // JSON にトラック 2 以上は含まれない
    assert!(
        !json.contains("\"track\": 2"),
        "空トラックが JSON に含まれている: {json}"
    );
    // 空小節 2 以降も含まれない
    assert!(
        !json.contains("\"meas\": 2"),
        "空小節が JSON に含まれている: {json}"
    );
    // 非空データは含まれる
    assert!(
        json.contains("t120"),
        "track0/meas0 の MML が含まれていない: {json}"
    );
    assert!(
        json.contains("cde"),
        "track1/meas1 の MML が含まれていない: {json}"
    );
}

#[test]
fn daw_save_json_track0_has_tempo_description() {
    // track0 のエントリに "tempo track" という description が付くこと
    let mut data = empty_data(TRACKS, MEASURES);
    let track_volumes = empty_track_volumes(TRACKS);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();

    let file = data_to_save_file(&data, &track_volumes, TRACKS, MEASURES);
    let json = serde_json::to_string_pretty(&file).unwrap();

    assert!(
        json.contains("\"tempo track\""),
        "track0 の description が JSON に含まれていない: {json}"
    );
}

#[test]
fn daw_save_json_meas0_has_initial_description() {
    // meas0 のエントリに "initial" という description が付くこと
    let mut data = empty_data(TRACKS, MEASURES);
    let track_volumes = empty_track_volumes(TRACKS);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();

    let file = data_to_save_file(&data, &track_volumes, TRACKS, MEASURES);
    let json = serde_json::to_string_pretty(&file).unwrap();

    assert!(
        json.contains("\"initial\""),
        "meas0 の description が JSON に含まれていない: {json}"
    );
}

#[test]
fn daw_save_json_non_initial_meas_has_no_description() {
    // meas1 以降のエントリには description が付かないこと（ムダな情報を書かない）
    let mut data = empty_data(TRACKS, MEASURES);
    let track_volumes = empty_track_volumes(TRACKS);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    data[crate::FIRST_PLAYABLE_TRACK][1] = "cde".to_string();

    let file = data_to_save_file(&data, &track_volumes, TRACKS, MEASURES);
    // DawSaveMeas の description フィールドを直接確認する
    let track1 = file.tracks.iter().find(|t| t.track == 1).unwrap();
    let meas1 = track1.meas.iter().find(|m| m.meas == 1).unwrap();
    assert!(
        meas1.description.is_none(),
        "meas1 に description が付いている: {:?}",
        meas1.description
    );
}

#[test]
fn daw_save_json_out_of_range_indices_are_ignored_on_load() {
    // JSON に含まれるトラック・小節インデックスが範囲外の場合は無視されること
    let json =
        r#"{"tracks":[{"track":100,"meas":[{"meas":0,"description":"initial","mml":"cde"}]}]}"#;
    let file: DawSaveFile = serde_json::from_str(json).unwrap();
    let mut data = empty_data(TRACKS, MEASURES);
    apply_save_file_to_data(&file, &mut data, TRACKS, MEASURES);
    // data は変更されていないこと
    for (t, row) in data.iter().enumerate().take(TRACKS) {
        for (m, cell) in row.iter().enumerate().take(MEASURES + 1) {
            assert!(
                cell.is_empty(),
                "範囲外インデックスが data を変更した: t={t}, m={m}"
            );
        }
    }
}

#[test]
fn required_grid_size_uses_largest_saved_track_and_measure() {
    let json =
        r#"{"tracks":[{"track":12,"meas":[{"meas":0,"mml":"t120"},{"meas":25,"mml":"cde"}]}]}"#;
    let file: DawSaveFile = serde_json::from_str(json).unwrap();

    // 返り値の 2 要素目は列数ではなく `DawApp.measures`（最大 measure index）。
    assert_eq!(required_grid_size(&file), (14, 25));
}

#[test]
fn load_saved_grid_size_reads_saved_session_from_history_dir() {
    let tmp = std::env::temp_dir().join("cmrt_test_daw_load_saved_grid_size");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();
    let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);
    let path = cmrt_history::daw_file_path().unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        r#"{"tracks":[{"track":10,"meas":[{"meas":0,"mml":"t120"},{"meas":18,"mml":"cde"}]}]}"#,
    )
    .unwrap();

    // load_saved_grid_size() は `DawApp.measures` にそのまま入る値を返す。
    assert_eq!(load_saved_grid_size(), Some((12, 18)));

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn daw_save_json_roundtrip_with_notes() {
    // 複数トラック・複数小節のデータが JSON 経由で正確に復元されること
    let mut data = empty_data(TRACKS, MEASURES);
    let mut track_volumes = empty_track_volumes(TRACKS);
    let mut restored_track_volumes = empty_track_volumes(TRACKS);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    data[2][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
    data[2][1] = "cde".to_string();
    data[2][2] = "efg".to_string();
    data[3][1] = "abc".to_string();
    track_volumes[2] = -6;
    track_volumes[3] = 3;

    let file = data_to_save_file(&data, &track_volumes, TRACKS, MEASURES);
    let json = serde_json::to_string_pretty(&file).unwrap();
    let loaded: DawSaveFile = serde_json::from_str(&json).unwrap();
    let mut restored = empty_data(TRACKS, MEASURES);
    apply_save_file_to_data(&loaded, &mut restored, TRACKS, MEASURES);
    apply_save_file_to_track_volumes(&loaded, &mut restored_track_volumes, TRACKS);

    assert_eq!(restored[0][0], data[0][0]);
    assert_eq!(restored[2][0], data[2][0]);
    assert_eq!(restored[2][1], data[2][1]);
    assert_eq!(restored[2][2], data[2][2]);
    assert_eq!(restored[3][1], data[3][1]);
    assert_eq!(restored_track_volumes[2], -6);
    assert_eq!(restored_track_volumes[3], 3);
    // 空セルは空のまま
    assert!(restored[4][1].is_empty());
}

#[test]
fn daw_save_json_keeps_track_volume_for_empty_track() {
    let data = empty_data(TRACKS, MEASURES);
    let mut track_volumes = empty_track_volumes(TRACKS);
    track_volumes[3] = -9;

    let file = data_to_save_file(&data, &track_volumes, TRACKS, MEASURES);
    let json = serde_json::to_string_pretty(&file).unwrap();

    assert!(
        json.contains("\"track\": 2"),
        "音量だけ変更したトラックが JSON に含まれていない: {json}"
    );
    assert!(
        json.contains("\"volume_db\": -9"),
        "track volume_db が JSON に含まれていない: {json}"
    );
}

#[test]
fn daw_load_clamps_track_volume_and_ignores_track0_volume() {
    let json = r#"{"tracks":[{"track":0,"volume_db":99,"meas":[]},{"track":1,"volume_db":-999,"meas":[]},{"track":2,"volume_db":99,"meas":[]}]}"#;
    let file: DawSaveFile = serde_json::from_str(json).unwrap();
    let mut track_volumes = empty_track_volumes(TRACKS);

    apply_save_file_to_track_volumes(&file, &mut track_volumes, TRACKS);

    assert_eq!(track_volumes[0], 0, "track0 volume should be ignored");
    assert_eq!(
        track_volumes[2], -36,
        "playable track volume should clamp to min"
    );
    assert_eq!(
        track_volumes[3], 6,
        "playable track volume should clamp to max"
    );
}

#[test]
fn daw_load_clears_defaults_before_applying_json() {
    // JSON が正常にパースできた場合、new() が設定したデフォルト値（data[0][0]）は
    // クリアされてから JSON の内容が適用されること。
    // これにより、ユーザーが track0/meas0 を空にして保存した場合に、次回起動で
    // デフォルト値が復活しないことを保証する。
    //
    // シミュレーション: 全セルが空の JSON（= ユーザーが全て消した状態）を
    // apply_save_file_to_data で適用すると、pre-populated なデフォルト値は
    // 上書きされず残ってしまうが、load() はクリア後に apply するため正しく空になる。
    let empty_file = DawSaveFile {
        tracks: vec![],
        chord_track: None,
    };
    let json = serde_json::to_string_pretty(&empty_file).unwrap();

    // data[0][0] にデフォルト値が入った状態を再現する
    let mut data = empty_data(TRACKS, MEASURES);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();

    // ① クリアなしで apply すると track0/meas0 はデフォルトのまま残る（バグの再現）
    {
        let loaded: DawSaveFile = serde_json::from_str(&json).unwrap();
        let mut data_no_clear = data.clone();
        apply_save_file_to_data(&loaded, &mut data_no_clear, TRACKS, MEASURES);
        assert_eq!(
            data_no_clear[0][0], DEFAULT_TRACK0_MML,
            "クリアなしでは空 JSON を適用してもデフォルト値が残る（バグの再現）"
        );
    }

    // ② クリアしてから apply するとデフォルト値は消える（修正後の正しい挙動）
    {
        let loaded: DawSaveFile = serde_json::from_str(&json).unwrap();
        for row in &mut data {
            for cell in row.iter_mut() {
                cell.clear();
            }
        }
        apply_save_file_to_data(&loaded, &mut data, TRACKS, MEASURES);
        assert!(
            data[0][0].is_empty(),
            "クリア後に空 JSON を適用すると track0/meas0 は空になるべき"
        );
    }
}
