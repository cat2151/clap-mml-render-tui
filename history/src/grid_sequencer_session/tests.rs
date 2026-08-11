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
        instances: vec![GridSequencerInstanceState {
            patch: Some("Keys/Piano.fxp".to_string()),
            lane_mode: GridLaneModeState::Single,
            drum: None,
            voicing_rotation: 0,
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
        pattern_evolution: GridPatternEvolutionState::Hold,
    };
    let json = serde_json::to_string(&state).unwrap();
    assert!(json.contains("\"instances\""));
    assert!(!json.contains("\"rows\""));
    assert!(json.contains("note_steps"));
    assert!(!json.contains("\"cells\""));
    assert!(!json.contains("\"duration\""));
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
        instances: vec![GridSequencerInstanceState {
            patch: Some("Bass/Poly.fxp".to_string()),
            lane_mode: GridLaneModeState::ChordVoices4,
            drum: None,
            voicing_rotation: -5,
            lanes: vec![GridSequencerLaneState::default(); CHORD_VOICE_LANES],
        }],
        pattern_evolution: GridPatternEvolutionState::Hold,
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
    assert_eq!(restored.pattern_evolution, GridPatternEvolutionState::Hold);
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
