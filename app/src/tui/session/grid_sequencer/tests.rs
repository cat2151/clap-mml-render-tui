use super::*;

#[test]
fn fixed_chord_text_round_trips_between_history_and_domain() {
    let history = crate::history::GridSequencerSessionState {
        instances: vec![crate::history::GridSequencerInstanceState::default()],
        cycle_random: crate::history::GridCycleRandomState {
            chord: false,
            ..crate::history::GridCycleRandomState::default()
        },
        fixed_chord: Some(crate::history::GridFixedChordState {
            input: "KEY:G♭ Isus4-I".to_string(),
        }),
    };

    let domain = grid_session_from_history(Some(history)).unwrap();
    assert_eq!(
        domain.fixed_chord.as_ref().unwrap().input(),
        "KEY:G♭ Isus4-I"
    );

    let restored = grid_session_to_history(Some(domain)).unwrap();
    assert_eq!(restored.fixed_chord.unwrap().input, "KEY:G♭ Isus4-I");
    assert!(!restored.cycle_random.chord);
}
