use super::*;

mod stale_cache;

fn song() -> DawGridImportSong {
    DawGridImportSong {
        bpm: 137.0,
        track_volumes_db: None,
        chord: None,
        tracks: vec![
            DawGridImportTrack {
                patch: Some("Keys/Piano.fxp".to_string()),
                swing: 61,
                measures: vec!["o5c4r4o5g2".to_string(), "o5d1".to_string()],
                chord_binding: None,
            },
            DawGridImportTrack {
                patch: None,
                swing: 50,
                measures: vec!["r1".to_string(), "o4c1".to_string()],
                chord_binding: None,
            },
        ],
    }
}

fn pattern(attacks: &[(usize, usize)]) -> Vec<DawGridNoteStep> {
    let mut steps = vec![DawGridNoteStep::Rest; 16];
    for &(start, duration) in attacks {
        steps[start] = DawGridNoteStep::Attack;
        for step in steps
            .iter_mut()
            .take((start + duration).min(16))
            .skip(start + 1)
        {
            *step = DawGridNoteStep::Tie;
        }
    }
    steps
}

fn lane(base_note: u8, attacks: &[(usize, usize)]) -> DawGridLane {
    DawGridLane {
        base_note,
        steps: pattern(attacks),
    }
}

fn chord_song() -> DawGridImportSong {
    DawGridImportSong {
        bpm: 120.0,
        track_volumes_db: None,
        chord: Some(DawGridChordSource {
            init: "key:C".to_string(),
            measures: vec!["I".to_string(), "V".to_string()],
            voicings: vec![
                DawGridChordVoicing {
                    bass: Some(48),
                    notes: vec![60, 64, 67],
                },
                DawGridChordVoicing {
                    bass: Some(43),
                    notes: vec![59, 62, 67],
                },
            ],
        }),
        tracks: vec![
            DawGridImportTrack {
                patch: Some("Pads/Chord.fxp".to_string()),
                swing: 50,
                measures: vec!["baked chord must not win".to_string(); 2],
                chord_binding: Some(DawGridChordBinding::Chord),
            },
            DawGridImportTrack {
                patch: Some("Bass/Mono.fxp".to_string()),
                swing: 50,
                measures: vec!["baked bass must not win".to_string(); 2],
                chord_binding: Some(DawGridChordBinding::Bass {
                    lanes: vec![lane(36, &[(0, 16)]), lane(48, &[])],
                }),
            },
            DawGridImportTrack {
                patch: Some("Keys/Arp.fxp".to_string()),
                swing: 50,
                measures: vec!["baked arp must not win".to_string(); 2],
                chord_binding: Some(DawGridChordBinding::Arpeggio {
                    rotation: 0,
                    lanes: vec![
                        lane(60, &[(0, 1)]),
                        lane(64, &[(4, 1)]),
                        lane(67, &[(8, 1)]),
                        lane(72, &[]),
                    ],
                }),
            },
        ],
    }
}

fn note_ons(snapshot: &DawProjectSnapshot, track: usize, measure: usize) -> Vec<(u32, u8)> {
    let mml =
        crate::mml::build_cell_mml_from_data(&snapshot.data, snapshot.measures, track, measure);
    crate::mml::tests::chord_note_counts::note_ons(&mml)
}

#[test]
fn daily_daw_is_fully_replaced_by_the_grid_song() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("grid_import_replace");
    let (mut app, _cache_rx) = crate::input::tests::build_test_app();
    app.workspace_kind = WorkspaceKind::Daily;
    app.editor.data[0][0] = "old tempo".to_string();
    app.editor.data[2][1] = "old song".to_string();

    app.replace_with_grid_song(song()).unwrap();

    assert_eq!(app.editor.tracks, FIRST_PLAYABLE_TRACK + 2);
    assert_eq!(app.editor.measures, 2);
    assert_eq!(app.editor.data[0][0], r#"{"beat":"4/4"}t137"#);
    assert_eq!(app.editor.data[1], vec!["", "", ""]);
    assert_eq!(app.editor.data[2][1], "o5c4r4o5g2");
    assert_eq!(app.editor.data[2][2], "o5d1");
    let init: Value = serde_json::from_str(&app.editor.data[2][0]).unwrap();
    assert_eq!(init["Surge XT patch"], "Keys/Piano.fxp");
    assert_eq!(init["swing"], 61);
    assert_eq!(app.editor.cursor_track, FIRST_PLAYABLE_TRACK);
    assert_eq!(app.editor.cursor_measure, 1);

    let rendered_mml = crate::mml::build_cell_mml_from_data(&app.editor.data, 2, 2, 1);
    assert_eq!(
        crate::mml::tests::chord_note_counts::note_ons(&rendered_mml),
        vec![(0, 60), (960, 67)]
    );
}

#[test]
fn persistent_daw_rejects_the_daily_only_import() {
    let (mut app, _cache_rx) = crate::input::tests::build_test_app();
    let before = app.editor.data.clone();

    let error = app.replace_with_grid_song(song()).unwrap_err();

    assert!(error.to_string().contains("Daily DAW"));
    assert_eq!(app.editor.data, before);
}

#[test]
fn chord_import_keeps_the_chord_track_and_live_generation_recipes() {
    let snapshot = grid_song_snapshot(chord_song()).unwrap();

    assert_eq!(snapshot.data[crate::CHORD_TRACK][0], "key:C");
    assert_eq!(snapshot.data[crate::CHORD_TRACK][1], "I");
    assert_eq!(snapshot.data[crate::CHORD_TRACK][2], "V");
    for track in FIRST_PLAYABLE_TRACK..snapshot.tracks {
        assert!(snapshot.data[track][1].is_empty());
        assert!(snapshot.data[track][2].is_empty());
        assert!(crate::mml::track_generates_from_chord_row(
            &snapshot.data,
            track
        ));
    }
    let chord_init: Value = serde_json::from_str(&snapshot.data[FIRST_PLAYABLE_TRACK][0]).unwrap();
    assert_eq!(
        chord_init["generate from chord track"]["binding"]["kind"],
        "chord"
    );
}

#[test]
fn unchanged_chord_uses_the_exact_grid_voicing_and_lane_patterns() {
    let snapshot = grid_song_snapshot(chord_song()).unwrap();
    let chord = FIRST_PLAYABLE_TRACK;
    let bass = chord + 1;
    let arp = chord + 2;

    assert_eq!(
        note_ons(&snapshot, chord, 1),
        vec![(0, 60), (0, 64), (0, 67)]
    );
    assert_eq!(note_ons(&snapshot, bass, 1), vec![(0, 48)]);
    assert_eq!(
        note_ons(&snapshot, arp, 1),
        vec![(0, 60), (480, 64), (960, 67)]
    );
}

#[test]
fn editing_the_chord_track_revoices_chord_bass_and_arp_together() {
    let mut snapshot = grid_song_snapshot(chord_song()).unwrap();
    snapshot.data[crate::CHORD_TRACK][1] = "IV".to_string();
    let chord = FIRST_PLAYABLE_TRACK;
    let bass = chord + 1;
    let arp = chord + 2;

    let chord_notes = note_ons(&snapshot, chord, 1);
    let chord_classes = chord_notes
        .iter()
        .map(|(_, note)| note % 12)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(chord_classes, [0, 5, 9].into_iter().collect());
    assert_eq!(note_ons(&snapshot, bass, 1)[0].1 % 12, 5);
    for (_, note) in note_ons(&snapshot, arp, 1) {
        assert!(chord_classes.contains(&(note % 12)));
    }
}

#[test]
fn a_handwritten_measure_detaches_only_that_cell_from_the_chord_track() {
    let mut snapshot = grid_song_snapshot(chord_song()).unwrap();
    let bass = FIRST_PLAYABLE_TRACK + 1;
    snapshot.data[bass][1] = "o7c1".to_string();
    snapshot.data[crate::CHORD_TRACK][1] = "IV".to_string();

    assert_eq!(note_ons(&snapshot, bass, 1), vec![(0, 84)]);
    assert!(!crate::mml::cell_is_generated_from_chord_row(
        &snapshot.data,
        bass,
        1
    ));
}

#[test]
fn project_recovery_keeps_the_chord_source_and_generation_recipe() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("grid_import_recovery");
    let (mut app, _cache_rx) = crate::input::tests::build_test_app();
    app.workspace_kind = WorkspaceKind::Daily;
    app.replace_with_grid_song(chord_song()).unwrap();

    let file = crate::project::project_file_from_app(&app);
    let restored = crate::project::project_snapshot_for_recovery(&file).unwrap();

    assert_eq!(restored.data[crate::CHORD_TRACK][1], "I");
    assert!(crate::mml::track_generates_from_chord_row(
        &restored.data,
        FIRST_PLAYABLE_TRACK
    ));
    assert_eq!(
        note_ons(&restored, FIRST_PLAYABLE_TRACK, 1),
        vec![(0, 60), (0, 64), (0, 67)]
    );
}
