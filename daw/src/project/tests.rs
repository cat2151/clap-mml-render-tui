use super::*;

fn sample_file() -> DawProjectFile {
    DawProjectFile {
        format: PROJECT_FORMAT.to_string(),
        format_version: PROJECT_FORMAT_VERSION,
        project: DawProjectData {
            track_count: 2,
            playable_measure_count: 2,
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
    assert_eq!(snapshot.tracks, 2);
    assert_eq!(snapshot.measures, 2);
    assert_eq!(snapshot.data[1][2], "cdef");
    assert_eq!(snapshot.track_volumes_db, vec![0, -6]);
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

    assert_eq!((preview.tracks, preview.measures), (2, 2));
    assert_eq!(preview.measure_index, Some(1));
    assert_eq!(preview.measure_samples, 176_400);
    assert!(preview.track_mmls[0].is_empty());
    assert!(preview.track_mmls[1].contains("t120"));
    assert!(preview.track_mmls[1].contains("cdef"));
    assert!((preview.track_gains[1] - 0.501_187_2).abs() < 0.000_001);
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
