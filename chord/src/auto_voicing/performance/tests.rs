use super::*;

fn progression() -> (TimedPerformance, Vec<ChordVoicing>) {
    let parsed = crate::parse_chord_progression("Key:C I-IV-V-I").unwrap();
    let voicings = crate::auto_voice_with_key(parsed.chords(), parsed.key_pitch_class(), None);
    let performance =
        crate::timed_chord_progression_performance(Some("Key:C"), "I-IV-V-I").unwrap();
    (performance, voicings)
}

fn messages(performance: &TimedPerformance, kind: u8) -> Vec<(f64, u8, u8)> {
    performance
        .events
        .iter()
        .filter(|event| event.message[0] & 0xf0 == kind)
        .map(|event| (event.seconds, event.message[1], event.message[2]))
        .collect()
}

fn note_on_groups(performance: &TimedPerformance) -> Vec<(f64, Vec<u8>)> {
    let mut groups: Vec<(f64, Vec<u8>)> = Vec::new();
    for event in &performance.events {
        if !is_note_on(event) {
            continue;
        }
        match groups.last_mut() {
            Some((seconds, notes)) if *seconds == event.seconds => notes.push(event.message[1]),
            _ => groups.push((event.seconds, vec![event.message[1]])),
        }
    }
    for (_, notes) in &mut groups {
        notes.sort_unstable();
    }
    groups
}

#[test]
fn timed_progression_keeps_its_rhythm_and_uses_the_selected_voicings() {
    let (original, voicings) = progression();
    let duration = original.duration_seconds;

    let voiced = revoice_timed_progression(original, &voicings, None).unwrap();

    assert_eq!(voiced.duration_seconds, duration);
    assert_eq!(
        note_on_groups(&voiced)
            .into_iter()
            .map(|(_, notes)| notes)
            .collect::<Vec<_>>(),
        voicings
            .iter()
            .map(|voicing| voicing.notes.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        voiced
            .events
            .iter()
            .filter(|event| is_note_on(event))
            .count(),
        voicings
            .iter()
            .map(|voicing| voicing.notes.len())
            .sum::<usize>(),
        "別パート用の bass は追加しない"
    );
}

#[test]
fn one_chord_keeps_the_voicing_chosen_in_the_whole_progression() {
    let (original, voicings) = progression();

    let selected = revoice_timed_progression(original, &voicings, Some(2)).unwrap();

    assert_eq!(
        note_on_groups(&selected),
        vec![(0.0, voicings[2].notes.clone())]
    );
    assert_eq!(selected.duration_seconds, 2.0);
    assert_eq!(selected.events.len(), voicings[2].notes.len() * 2);
}

#[test]
fn bass_uses_each_i_iv_v_i_pitch_once_without_changing_the_chords() {
    let (mut original, voicings) = progression();
    let mut onset = None;
    let mut velocity = 72;
    for event in &mut original.events {
        if is_note_on(event) && onset != Some(event.seconds) {
            onset = Some(event.seconds);
            event.message[2] = velocity;
            velocity += 1;
        }
    }
    let unchanged = original.clone();

    let bass = bass_timed_progression(&original, &voicings, None).unwrap();

    assert_eq!(original, unchanged, "Chord performance は変更しない");
    assert_eq!(bass.duration_seconds, original.duration_seconds);
    assert_eq!(bass.from_chord, original.from_chord);
    assert_eq!(
        messages(&bass, NOTE_ON),
        voicings
            .iter()
            .enumerate()
            .map(|(index, voicing)| {
                (
                    index as f64 * 2.0,
                    voicing.bass.expect("catalog chord has a bass"),
                    72 + index as u8,
                )
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        messages(&bass, NOTE_OFF),
        voicings
            .iter()
            .enumerate()
            .map(|(index, voicing)| {
                (
                    (index + 1) as f64 * 2.0,
                    voicing.bass.expect("catalog chord has a bass"),
                    0,
                )
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(bass.events.len(), voicings.len() * 2);
}

#[test]
fn selected_bass_starts_at_zero_and_keeps_that_chords_duration() {
    let (original, voicings) = progression();
    let source_velocity = chord_intervals(&original).unwrap()[2].note_on.message[2];

    let bass = bass_timed_progression(&original, &voicings, Some(2)).unwrap();

    assert_eq!(
        messages(&bass, NOTE_ON),
        vec![(0.0, voicings[2].bass.unwrap(), source_velocity)]
    );
    assert_eq!(
        messages(&bass, NOTE_OFF),
        vec![(2.0, voicings[2].bass.unwrap(), 0)]
    );
    assert_eq!(bass.duration_seconds, 2.0);
}

#[test]
fn a_voicing_without_bass_is_silent_for_its_original_interval() {
    let (original, mut voicings) = progression();
    voicings[1].bass = None;

    let full = bass_timed_progression(&original, &voicings, None).unwrap();
    let selected = bass_timed_progression(&original, &voicings, Some(1)).unwrap();

    assert_eq!(messages(&full, NOTE_ON).len(), 3);
    assert_eq!(messages(&full, NOTE_OFF).len(), 3);
    assert_eq!(full.duration_seconds, original.duration_seconds);
    assert!(selected.events.is_empty());
    assert_eq!(selected.duration_seconds, 2.0);
}

#[test]
fn adjacent_equal_bass_pitches_are_released_before_they_are_retriggered() {
    let parsed = crate::parse_chord_progression("Key:C I-I").unwrap();
    let voicings = crate::auto_voice_with_key(parsed.chords(), parsed.key_pitch_class(), None);
    let original = crate::timed_chord_progression_performance(Some("Key:C"), "I-I").unwrap();

    let bass = bass_timed_progression(&original, &voicings, None).unwrap();

    assert_eq!(bass.events.len(), 4);
    assert!(is_note_off(&bass.events[1]));
    assert!(is_note_on(&bass.events[2]));
    assert_eq!(bass.events[1].seconds, bass.events[2].seconds);
    assert_eq!(bass.events[1].message[1], bass.events[2].message[1]);
}

#[test]
fn mismatched_chord_and_voicing_counts_return_errors() {
    let (original, voicings) = progression();

    let too_few = bass_timed_progression(&original, &voicings[..3], None).unwrap_err();
    let mut too_many = voicings.clone();
    too_many.push(voicings[0].clone());
    let too_many = bass_timed_progression(&original, &too_many, None).unwrap_err();

    assert!(too_few.contains("一致しません"), "{too_few}");
    assert!(too_many.contains("一致しません"), "{too_many}");
}
