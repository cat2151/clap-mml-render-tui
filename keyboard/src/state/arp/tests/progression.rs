use super::*;

#[test]
fn arp_replaces_only_after_every_chord_sequence_and_starts_the_new_lowest_note() {
    for mode in [NotePlaybackMode::Arp, NotePlaybackMode::Auto] {
        for initial in [vec![vec![64, 60], vec![65]], vec![vec![60]]] {
            let mut state = KeyboardState::default();
            let origin = Instant::now();
            let old_notes = initial.iter().map(Vec::len).sum::<usize>() * 2;
            state.set_detected_voicing(PatchVoicing::Mono);
            state.replace_repeat_chords(initial.clone(), origin, false);
            if mode == NotePlaybackMode::Arp {
                enter_arp(&mut state, origin);
            } else {
                state.cycle_note_playback(origin);
            }
            let generation = state.periodic_generation();
            let mut calls = 0;
            for step in 1..=old_notes + 6 {
                let at = origin + PERIODIC_INTERVAL * step as u32;
                state.apply_scheduled_progression(at - Duration::from_millis(200));
                let old_note = state.arp_sounding.unwrap().midi_note;
                let tick = state
                    .poll_periodic_tick_with_progression(at, |_| {
                        calls += 1;
                        Some(vec![vec![71, 67, 62]])
                    })
                    .unwrap();
                assert_eq!(
                    calls,
                    usize::from(step >= old_notes) + usize::from(step >= old_notes + 6)
                );
                assert_eq!(tick.messages[0], [0x80, old_note, 0]);
                if step == old_notes {
                    assert_eq!(tick.messages, vec![[0x80, old_note, 0], [0x90, 62, 100]]);
                    assert_eq!(chord_notes(&state), initial);
                    let before = state
                        .sounding_position(at - Duration::from_millis(1))
                        .unwrap();
                    assert_eq!(before.chord_index, initial.len() - 1);
                    assert_eq!(before.arp.unwrap().note.midi_note, old_note);
                    let after = state.sounding_position(at).unwrap();
                    assert_eq!(after.chord_index, 0);
                    assert_eq!(after.arp.unwrap().index, 0);
                    assert_eq!(after.arp.unwrap().note.midi_note, 62);
                }
                assert_eq!(state.periodic_generation(), generation);
                assert_eq!(state.periodic_anchor(), Some(origin));
            }
        }
    }
}
