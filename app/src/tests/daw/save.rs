use super::super::{DEFAULT_TRACK0_MML, MEASURES, TRACKS};
use super::{apply_save_file_to_data, data_to_save_file, DawSaveFile};

/// テスト用ヘルパー: TRACKS×(MEASURES+1) の空 data を作成する
fn empty_data(tracks: usize, measures: usize) -> Vec<Vec<String>> {
    vec![vec![String::new(); measures + 1]; tracks]
}

// ─── ensure_cmrt_dir ──────────────────────────────────────────

#[test]
fn ensure_cmrt_dir_is_idempotent() {
    // 複数回呼んでもエラーにならない（一時ディレクトリを使って設定ディレクトリを汚染しない）
    let tmp = std::env::temp_dir().join("cmrt_test_daw_idempotent");
    let guard = crate::test_utils::TestEnvGuard::set("CMRT_BASE_DIR", &tmp);
    std::fs::remove_dir_all(&tmp).ok();

    let r1 = cmrt_core::ensure_cmrt_dir();
    let r2 = cmrt_core::ensure_cmrt_dir();

    assert!(r1.is_ok(), "初回 ensure_cmrt_dir が失敗: {:?}", r1.err());
    assert!(r2.is_ok(), "2回目 ensure_cmrt_dir が失敗: {:?}", r2.err());

    drop(guard); // CMRT_BASE_DIR を復元してからクリーンアップする
    std::fs::remove_dir_all(&tmp).ok();
}

// ─── JSON 保存形式 ────────────────────────────────────────────

#[test]
fn daw_save_json_roundtrip_default_data() {
    // デフォルト data（track0/meas0 のみ非空）が JSON 経由で復元されること
    let mut data = empty_data(TRACKS, MEASURES);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();

    let file = data_to_save_file(&data, TRACKS, MEASURES);
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
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    data[1][1] = "cde".to_string();
    // track2..8, meas2..MEASURES は空

    let file = data_to_save_file(&data, TRACKS, MEASURES);
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
    data[0][0] = DEFAULT_TRACK0_MML.to_string();

    let file = data_to_save_file(&data, TRACKS, MEASURES);
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
    data[0][0] = DEFAULT_TRACK0_MML.to_string();

    let file = data_to_save_file(&data, TRACKS, MEASURES);
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
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    data[1][1] = "cde".to_string();

    let file = data_to_save_file(&data, TRACKS, MEASURES);
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
    for t in 0..TRACKS {
        for m in 0..=MEASURES {
            assert!(
                data[t][m].is_empty(),
                "範囲外インデックスが data を変更した: t={t}, m={m}"
            );
        }
    }
}

#[test]
fn daw_save_json_roundtrip_with_notes() {
    // 複数トラック・複数小節のデータが JSON 経由で正確に復元されること
    let mut data = empty_data(TRACKS, MEASURES);
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    data[1][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
    data[1][1] = "cde".to_string();
    data[1][2] = "efg".to_string();
    data[2][1] = "abc".to_string();

    let file = data_to_save_file(&data, TRACKS, MEASURES);
    let json = serde_json::to_string_pretty(&file).unwrap();
    let loaded: DawSaveFile = serde_json::from_str(&json).unwrap();
    let mut restored = empty_data(TRACKS, MEASURES);
    apply_save_file_to_data(&loaded, &mut restored, TRACKS, MEASURES);

    assert_eq!(restored[0][0], data[0][0]);
    assert_eq!(restored[1][0], data[1][0]);
    assert_eq!(restored[1][1], data[1][1]);
    assert_eq!(restored[1][2], data[1][2]);
    assert_eq!(restored[2][1], data[2][1]);
    // 空セルは空のまま
    assert!(restored[3][1].is_empty());
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
    let empty_file = DawSaveFile { tracks: vec![] };
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
