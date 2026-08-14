use super::*;

fn kinds(values: &[GridNoteStepState]) -> String {
    values
        .iter()
        .map(|value| match value {
            GridNoteStepState::Rest => '.',
            GridNoteStepState::Attack => '#',
            GridNoteStepState::Tie => '-',
        })
        .collect()
}

#[test]
fn new_format_round_trip_preserves_grid_values_and_omits_legacy_fields() {
    let state = GridSequencerSessionState {
        fixed_chord: None,
        instances: vec![GridSequencerInstanceState {
            patch: Some("Keys/Piano.fxp".to_string()),
            lane_mode: GridLaneModeState::Single,
            drum: None,
            voicing_rotation: 0,
            swing: SWING_MIN,
            lanes: vec![GridSequencerLaneState {
                base_note: 64,
                note_steps: vec![
                    GridNoteStepState::Attack,
                    GridNoteStepState::Tie,
                    GridNoteStepState::Rest,
                    GridNoteStepState::Attack,
                ],
            }],
        }],
        cycle_random: GridCycleRandomState {
            patch: false,
            note: false,
            drum: false,
            arp: false,
            chord: true,
            bpm: true,
            swing: false,
        },
    };
    let json = serde_json::to_string(&state).unwrap();
    assert!(json.contains("\"instances\""));
    assert!(!json.contains("\"rows\""));
    assert!(json.contains("note_steps"));
    assert!(!json.contains("\"cells\""));
    assert!(!json.contains("\"duration\""));
    assert!(!json.contains("\"fixed_chord\""));
    let restored: GridSequencerSessionState = serde_json::from_str(&json).unwrap();
    let mut expected = state;
    expected.instances[0].lanes[0]
        .note_steps
        .resize(GRID_STEPS, GridNoteStepState::Rest);
    assert_eq!(restored, expected);
}

#[test]
fn downward_voicing_rotation_round_trips_as_a_negative_value() {
    let state = GridSequencerSessionState {
        fixed_chord: None,
        instances: vec![GridSequencerInstanceState {
            patch: Some("Bass/Poly.fxp".to_string()),
            lane_mode: GridLaneModeState::ChordVoices4,
            drum: None,
            voicing_rotation: -5,
            swing: 62,
            lanes: vec![GridSequencerLaneState::default(); CHORD_VOICE_LANES],
        }],
        cycle_random: GridCycleRandomState {
            patch: false,
            note: false,
            drum: false,
            arp: false,
            chord: true,
            bpm: true,
            swing: false,
        },
    };

    let json = serde_json::to_string(&state).unwrap();
    let restored: GridSequencerSessionState = serde_json::from_str(&json).unwrap();

    assert!(json.contains("\"voicing_rotation\":-5"));
    assert_eq!(restored.instances[0].voicing_rotation, -5);
}

#[test]
fn legacy_rows_migrate_to_instances_and_expand_the_second_row() {
    let restored: GridSequencerSessionState = serde_json::from_str(
        r#"{"rows":[{"patch":"Chord.fxp","base_note":48,"note_steps":["attack"]},{"patch":"Bass.fxp","base_note":52,"note_steps":["attack","tie"]},{"base_note":67,"note_steps":[]}],"pattern_evolution":"hold"}"#,
    )
    .unwrap();

    assert_eq!(restored.instances.len(), 3);
    assert_eq!(restored.instances[0].lane_mode, GridLaneModeState::Single);
    assert_eq!(
        restored.instances[1].lane_mode,
        GridLaneModeState::ChordVoices4
    );
    assert_eq!(restored.instances[1].lanes.len(), CHORD_VOICE_LANES);
    assert_eq!(restored.instances[1].voicing_rotation, 0);
    assert_eq!(restored.instances[1].lanes[0].base_note, 52);
    assert_eq!(
        kinds(&restored.instances[1].lanes[0].note_steps),
        "#-.............."
    );
    assert!(restored.instances[1].lanes[1..]
        .iter()
        .all(|lane| lane == &GridSequencerLaneState::default()));
    assert_eq!(
        restored.cycle_random,
        GridCycleRandomState {
            patch: false,
            note: false,
            drum: false,
            arp: false,
            chord: true,
            bpm: true,
            swing: false,
        },
        "旧 HOLD は譜面まわりの4項目だけ OFF へ移行する"
    );
}

#[test]
fn instances_field_has_priority_over_legacy_rows_even_when_empty() {
    let restored: GridSequencerSessionState = serde_json::from_str(
        r#"{"instances":[],"rows":[{"base_note":72,"note_steps":["attack"]}]}"#,
    )
    .unwrap();

    assert!(restored.instances.is_empty());
}

#[test]
fn chord_voice_instances_normalize_lane_and_step_counts() {
    let restored: GridSequencerSessionState = serde_json::from_str(
        r#"{"instances":[{"patch":"Mono/Bass.fxp","lane_mode":"chord_voices4","voicing_rotation":6,"lanes":[{"base_note":40,"note_steps":["tie","attack","tie"]},{"base_note":41,"note_steps":["attack","attack","attack","attack","attack","attack","attack","attack","attack","attack","attack","attack","attack","attack","attack","attack","attack"]}]}]}"#,
    )
    .unwrap();

    let instance = &restored.instances[0];
    assert_eq!(instance.patch.as_deref(), Some("Mono/Bass.fxp"));
    assert_eq!(instance.voicing_rotation, 6);
    assert_eq!(instance.lanes.len(), CHORD_VOICE_LANES);
    assert_eq!(kinds(&instance.lanes[0].note_steps), ".#-.............");
    assert_eq!(instance.lanes[1].note_steps.len(), GRID_STEPS);
    assert_eq!(instance.lanes[2], GridSequencerLaneState::default());
}

#[test]
fn malformed_new_steps_are_normalized_and_have_priority_over_legacy() {
    let restored: GridSequencerRowState = serde_json::from_str(
        r#"{"patch":123,"base_note":999,"note_steps":["tie","future","attack","tie","rest","tie"],"duration":"quarter","cells":[true,true,true]}"#,
    )
    .unwrap();
    assert_eq!(restored.patch, None);
    assert_eq!(restored.base_note, 127);
    assert_eq!(&kinds(&restored.note_steps)[..6], "..#-..");
    assert_eq!(restored.note_steps.len(), GRID_STEPS);
}

#[test]
fn short_and_long_new_lists_are_always_sixteen_steps() {
    let short: GridSequencerRowState =
        serde_json::from_str(r#"{"note_steps":["attack"]}"#).unwrap();
    let long_json = format!(r#"{{"note_steps":[{}]}}"#, vec!["\"attack\""; 20].join(","));
    let long: GridSequencerRowState = serde_json::from_str(&long_json).unwrap();
    assert_eq!(kinds(&short.note_steps), "#...............");
    assert_eq!(kinds(&long.note_steps), "################");
}

#[test]
fn all_rest_new_field_is_not_mistaken_for_an_absent_field() {
    let row: GridSequencerRowState =
        serde_json::from_str(r#"{"note_steps":[],"duration":"quarter","cells":[true]}"#).unwrap();
    assert_eq!(kinds(&row.note_steps), "................");
}

#[test]
fn legacy_sixteenth_keeps_each_active_cell_as_an_attack() {
    let row: GridSequencerRowState =
        serde_json::from_str(r#"{"duration":"sixteenth","cells":[true,false,true]}"#).unwrap();
    assert_eq!(&kinds(&row.note_steps)[..4], "#.#.");
}

#[test]
fn legacy_quarter_extends_until_the_next_attack_and_clamps_at_the_bar() {
    let row: GridSequencerRowState = serde_json::from_str(
        r#"{"duration":"quarter","cells":[true,false,false,false,true,false,true,true]}"#,
    )
    .unwrap();
    assert_eq!(&kinds(&row.note_steps)[..8], "#---#-##");
    let tail: GridSequencerRowState = serde_json::from_str(
        r#"{"duration":"quarter","cells":[false,false,false,false,false,false,false,false,false,false,false,false,false,false,true,false]}"#,
    )
    .unwrap();
    assert_eq!(&kinds(&tail.note_steps)[14..], "#-");
}

/// `cycle_random` を知らない頃のセッションは、AUTO / HOLD から移行する。
#[test]
fn legacy_pattern_evolution_migrates_into_cycle_random() {
    let auto: GridSequencerSessionState =
        serde_json::from_str(r#"{"instances":[],"pattern_evolution":"auto"}"#).unwrap();
    assert_eq!(auto.cycle_random, GridCycleRandomState::default());

    // field ごと無い（さらに古い）セッションも既定の全 ON で読む。
    let missing: GridSequencerSessionState = serde_json::from_str(r#"{"instances":[]}"#).unwrap();
    assert_eq!(missing.cycle_random, GridCycleRandomState::default());
}

/// 項目を足したあとに古い版が書き戻した JSON でも、足した項目が黙って OFF にならない。
#[test]
fn a_partial_cycle_random_object_keeps_the_missing_items_on() {
    let restored: GridSequencerSessionState =
        serde_json::from_str(r#"{"instances":[],"cycle_random":{"patch":false}}"#).unwrap();

    assert_eq!(
        restored.cycle_random,
        GridCycleRandomState {
            patch: false,
            ..GridCycleRandomState::default()
        }
    );
}

#[test]
fn swing_round_trips_and_defaults_to_no_shuffle_when_missing() {
    let restored: GridSequencerSessionState = serde_json::from_str(
        r#"{"instances":[{"patch":null,"lane_mode":"single","voicing_rotation":0,"swing":63,"lanes":[]},{"patch":null,"lane_mode":"single","voicing_rotation":0,"lanes":[]}]}"#,
    )
    .unwrap();

    assert_eq!(restored.instances[0].swing, 63);
    // swing を知らない頃のセッション。跳ねなしから始める。
    assert_eq!(restored.instances[1].swing, SWING_MIN);
}

#[test]
fn fixed_chord_source_text_round_trips_without_validation() {
    let state = GridSequencerSessionState {
        fixed_chord: Some(GridFixedChordState {
            input: "KEY:G♭ Isus4-I".to_string(),
        }),
        ..GridSequencerSessionState::default()
    };

    let json = serde_json::to_string(&state).unwrap();
    let restored: GridSequencerSessionState = serde_json::from_str(&json).unwrap();

    assert!(json.contains("\"fixed_chord\""));
    assert_eq!(restored.fixed_chord, state.fixed_chord);

    let invalid: GridSequencerSessionState =
        serde_json::from_str(r#"{"instances":[],"fixed_chord":{"input":"not valid yet"}}"#)
            .unwrap();
    assert_eq!(invalid.fixed_chord.unwrap().input, "not valid yet");
}

#[test]
fn an_out_of_range_swing_is_clamped_instead_of_wrapping() {
    let restored: GridSequencerSessionState = serde_json::from_str(
        r#"{"instances":[{"lane_mode":"single","swing":900,"lanes":[]},{"lane_mode":"single","swing":-4,"lanes":[]}]}"#,
    )
    .unwrap();

    assert_eq!(restored.instances[0].swing, SWING_MAX);
    assert_eq!(restored.instances[1].swing, SWING_MIN);
}

/// 項目を知らない版が書き戻したセッションでも、SWING が黙って OFF にならない。
#[test]
fn a_cycle_random_without_the_swing_flag_keeps_it_on() {
    let restored: GridSequencerSessionState = serde_json::from_str(
        r#"{"instances":[],"cycle_random":{"patch":false,"note":true,"drum":true,"arp":true,"chord":true,"bpm":true}}"#,
    )
    .unwrap();

    assert!(restored.cycle_random.swing);
    assert!(!restored.cycle_random.patch);
}

/// 旧 HOLD は「据え置き」の意図なので、swing も一緒に止める。
#[test]
fn the_legacy_hold_migration_turns_swing_off() {
    let restored: GridSequencerSessionState =
        serde_json::from_str(r#"{"instances":[],"pattern_evolution":"hold"}"#).unwrap();

    assert!(!restored.cycle_random.swing);
    assert!(restored.cycle_random.chord);
}
