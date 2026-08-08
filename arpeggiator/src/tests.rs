use super::{generate, ArpPattern, DURATION_CHOICES};

const STEPS: usize = 16;

#[test]
fn every_step_gets_exactly_one_note_in_order() {
    for pattern in ArpPattern::ALL {
        let notes = generate(pattern, 4, STEPS, &mut rand::rng());
        assert_eq!(notes.len(), STEPS, "{}", pattern.label());
        for (step, note) in notes.iter().enumerate() {
            assert_eq!(note.step, step, "{}", pattern.label());
        }
    }
}

#[test]
fn voices_stay_inside_the_available_range() {
    for pattern in ArpPattern::ALL {
        for voice_count in 1..=8 {
            let notes = generate(pattern, voice_count, STEPS, &mut rand::rng());
            assert!(
                notes.iter().all(|note| note.voice < voice_count),
                "{} left the range of {voice_count} voices",
                pattern.label()
            );
        }
    }
}

#[test]
fn durations_come_from_the_choices_and_never_overlap_the_same_voice() {
    for pattern in ArpPattern::ALL {
        let notes = generate(pattern, 4, STEPS, &mut rand::rng());
        for note in &notes {
            assert!(note.duration_steps >= 1, "{}", pattern.label());
            assert!(
                note.duration_steps <= DURATION_CHOICES.into_iter().max().unwrap(),
                "{}",
                pattern.label()
            );
            assert!(
                note.step + note.duration_steps <= STEPS,
                "{}",
                pattern.label()
            );
            let overlapped = notes[note.step + 1..note.step + note.duration_steps]
                .iter()
                .any(|next| next.voice == note.voice);
            assert!(
                !overlapped,
                "{} let voice {} overlap its own next attack at step {}",
                pattern.label(),
                note.voice,
                note.step
            );
        }
    }
}

#[test]
fn a_deterministic_pattern_repeats_its_period() {
    let notes = generate(ArpPattern::Up, 4, STEPS, &mut rand::rng());
    let voices = notes.iter().map(|note| note.voice).collect::<Vec<_>>();
    assert_eq!(voices, [0, 1, 2, 3].repeat(4));
}

#[test]
fn empty_inputs_produce_nothing() {
    assert!(generate(ArpPattern::Up, 0, STEPS, &mut rand::rng()).is_empty());
    assert!(generate(ArpPattern::Up, 4, 0, &mut rand::rng()).is_empty());
    assert!(generate(ArpPattern::Random, 0, STEPS, &mut rand::rng()).is_empty());
}

#[test]
fn a_single_voice_still_produces_one_step_notes() {
    let notes = generate(ArpPattern::Random, 1, STEPS, &mut rand::rng());
    assert_eq!(notes.len(), STEPS);
    assert!(notes.iter().all(|note| note.voice == 0));
    // 同じ声部が毎 step 来るので、打ち切りで必ず1 stepになる。
    assert!(notes.iter().all(|note| note.duration_steps == 1));
}
