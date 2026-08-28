use super::*;

fn sample_file() -> DawProjectFile {
    DawProjectFile {
        format: PROJECT_FORMAT.to_string(),
        format_version: PROJECT_FORMAT_VERSION,
        project: DawProjectData {
            track_count: 2,
            playable_measure_count: 2,
            chord_track: None,
            tracks: vec![
                DawProjectTrack {
                    track_index: 0,
                    role: DawProjectTrackRole::GlobalHeader,
                    volume_db: 0,
                    non_empty_cells: vec![DawProjectCell {
                        measure_index: 0,
                        role: DawProjectCellRole::Initialization,
                        mml: r#"{"beat":"4/4"}t120"#.to_string(),
                    }],
                },
                DawProjectTrack {
                    track_index: 1,
                    role: DawProjectTrackRole::Instrument,
                    volume_db: -6,
                    non_empty_cells: vec![DawProjectCell {
                        measure_index: 2,
                        role: DawProjectCellRole::PlayableMeasure,
                        mml: "cdef".to_string(),
                    }],
                },
            ],
        },
    }
}

#[test]
fn project_json_is_self_identifying_and_roundtrips() {
    let json = serde_json::to_string_pretty(&sample_file()).unwrap();

    assert!(json.contains(r#""format": "clap-mml-render-tui.daw-project""#));
    assert!(json.contains(r#""format_version": 1"#));
    assert!(json.contains(r#""role": "global_header""#));
    assert!(json.contains(r#""role": "playable_measure""#));

    let loaded = serde_json::from_str(&json).unwrap();
    let snapshot = validate_project_file(&loaded).unwrap();
    // 保存ファイルの track_count は chord 行を含まない（2）。
    // グリッドは chord 行のぶん 1 行増える。
    assert_eq!(snapshot.tracks, 3);
    assert_eq!(snapshot.measures, 2);
    assert_eq!(snapshot.data[crate::FIRST_PLAYABLE_TRACK][2], "cdef");
    assert_eq!(snapshot.track_volumes_db, vec![0, 0, -6]);
}

#[test]
fn project_rejects_unknown_format_without_partial_interpretation() {
    let mut file = sample_file();
    file.format = "some-other-format".to_string();

    let error = validate_project_file(&file).err().unwrap().to_string();

    assert!(error.contains("project format が違います"));
}

#[test]
fn project_rejects_duplicate_or_out_of_order_coordinates() {
    let mut file = sample_file();
    file.project.tracks[1].non_empty_cells.insert(
        0,
        DawProjectCell {
            measure_index: 2,
            role: DawProjectCellRole::PlayableMeasure,
            mml: "g".to_string(),
        },
    );

    let error = validate_project_file(&file).err().unwrap().to_string();

    assert!(error.contains("measure_index 順に重複なく"));
}

#[test]
fn project_rejects_role_that_disagrees_with_coordinate() {
    let mut file = sample_file();
    file.project.tracks[1].non_empty_cells[0].role = DawProjectCellRole::Initialization;

    let error = validate_project_file(&file).err().unwrap().to_string();

    assert!(error.contains("role が index と一致しません"));
}

#[test]
fn project_json_rejects_unknown_fields() {
    let json = r#"{
        "format":"clap-mml-render-tui.daw-project",
        "format_version":1,
        "project":{"track_count":2,"playable_measure_count":1,"tracks":[],"typo":true}
    }"#;

    let error = serde_json::from_str::<DawProjectFile>(json)
        .err()
        .unwrap()
        .to_string();

    assert!(error.contains("unknown field"));
}

#[test]
fn preview_inspection_uses_first_playable_measure_without_applying_project() {
    let snapshot = validate_project_file(&sample_file()).unwrap();

    let preview = preview_from_snapshot(&snapshot, 44_100.0);

    assert_eq!((preview.tracks, preview.measures), (3, 2));
    assert_eq!(preview.measure_index, Some(1));
    assert_eq!(preview.measure_samples, 176_400);
    assert!(preview.track_mmls[0].is_empty());
    assert!(preview.track_mmls[crate::CHORD_TRACK].is_empty());
    assert!(preview.track_mmls[crate::FIRST_PLAYABLE_TRACK].contains("t120"));
    assert!(preview.track_mmls[crate::FIRST_PLAYABLE_TRACK].contains("cdef"));
    assert!((preview.track_gains[crate::FIRST_PLAYABLE_TRACK] - 0.501_187_2).abs() < 0.000_001);
}

#[test]
fn preview_inspection_accepts_project_without_playable_cells() {
    let mut file = sample_file();
    file.project.tracks[1].non_empty_cells.clear();
    let snapshot = validate_project_file(&file).unwrap();

    let preview = preview_from_snapshot(&snapshot, 44_100.0);

    assert_eq!(preview.measure_index, None);
    assert!(preview.track_mmls.iter().all(String::is_empty));
}

// ─── chord 行 ────────────────────────────────────────────────

/// chord 行は `tracks` ではなく専用フィールドに置き、グリッドの chord 行へ戻る。
#[test]
fn the_chord_track_field_loads_into_the_chord_row() {
    let mut file = sample_file();
    file.project.chord_track = Some(DawProjectChordTrack {
        non_empty_cells: vec![
            DawProjectCell {
                measure_index: 0,
                role: DawProjectCellRole::Initialization,
                mml: "key:G".to_string(),
            },
            DawProjectCell {
                measure_index: 2,
                role: DawProjectCellRole::PlayableMeasure,
                mml: "I-IV-V-I".to_string(),
            },
        ],
    });

    let snapshot = validate_project_file(&file).unwrap();

    assert_eq!(snapshot.data[crate::CHORD_TRACK][0], "key:G");
    assert_eq!(snapshot.data[crate::CHORD_TRACK][2], "I-IV-V-I");
    // 演奏 track の中身は chord 行に吸われない
    assert_eq!(snapshot.data[crate::FIRST_PLAYABLE_TRACK][2], "cdef");
}

/// chord 行を持たない project file は、chord 行が空のグリッドとして読める。
#[test]
fn a_project_file_without_a_chord_track_leaves_the_chord_row_empty() {
    let snapshot = validate_project_file(&sample_file()).unwrap();

    assert!(snapshot.data[crate::CHORD_TRACK]
        .iter()
        .all(|cell| cell.is_empty()));
}
