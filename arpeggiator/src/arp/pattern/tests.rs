use super::{up_down_sequence, ArpPattern};

fn sequence(pattern: ArpPattern, voice_count: usize) -> Vec<usize> {
    pattern
        .voice_sequence(voice_count)
        .expect("deterministic pattern has a period")
}

#[test]
fn four_voice_sequences_match_the_documented_shapes() {
    assert_eq!(sequence(ArpPattern::Up, 4), [0, 1, 2, 3]);
    assert_eq!(sequence(ArpPattern::Down, 4), [3, 2, 1, 0]);
    assert_eq!(sequence(ArpPattern::UpDown, 4), [0, 1, 2, 3, 2, 1]);
    assert_eq!(sequence(ArpPattern::DownUp, 4), [3, 2, 1, 0, 1, 2]);
    assert_eq!(
        sequence(ArpPattern::UpDownHold, 4),
        [0, 1, 2, 3, 3, 2, 1, 0]
    );
    assert_eq!(sequence(ArpPattern::Converge, 4), [0, 3, 1, 2]);
    assert_eq!(sequence(ArpPattern::Diverge, 4), [1, 2, 0, 3]);
    assert_eq!(sequence(ArpPattern::Octave, 4), [0, 3, 1, 3, 2, 3]);
}

#[test]
fn odd_voice_counts_visit_the_centre_once() {
    assert_eq!(sequence(ArpPattern::Converge, 5), [0, 4, 1, 3, 2]);
    assert_eq!(sequence(ArpPattern::Diverge, 5), [2, 1, 3, 0, 4]);
    assert_eq!(sequence(ArpPattern::Converge, 3), [0, 2, 1]);
    assert_eq!(sequence(ArpPattern::Diverge, 3), [1, 0, 2]);
}

#[test]
fn every_deterministic_sequence_stays_in_range_and_is_non_empty() {
    for voice_count in 1..=8 {
        for pattern in ArpPattern::ALL {
            let Some(period) = pattern.voice_sequence(voice_count) else {
                assert_eq!(pattern, ArpPattern::Random);
                continue;
            };
            assert!(
                !period.is_empty(),
                "{} produced an empty period for {voice_count} voices",
                pattern.label()
            );
            assert!(
                period.iter().all(|voice| *voice < voice_count),
                "{} left the voice range for {voice_count} voices: {period:?}",
                pattern.label()
            );
        }
    }
}

#[test]
fn every_deterministic_sequence_covers_all_voices() {
    // どの音型でも、1周すれば全声部が最低1回は鳴る（声部が余ると和音が痩せる）。
    for voice_count in 2..=8 {
        for pattern in ArpPattern::ALL {
            let Some(period) = pattern.voice_sequence(voice_count) else {
                continue;
            };
            for voice in 0..voice_count {
                assert!(
                    period.contains(&voice),
                    "{} skipped voice {voice} of {voice_count}: {period:?}",
                    pattern.label()
                );
            }
        }
    }
}

#[test]
fn two_voices_degrade_without_panicking() {
    assert_eq!(sequence(ArpPattern::UpDown, 2), [0, 1]);
    assert_eq!(sequence(ArpPattern::DownUp, 2), [1, 0]);
    assert_eq!(sequence(ArpPattern::Octave, 2), [0, 1]);
    assert_eq!(sequence(ArpPattern::Up, 1), [0]);
    assert_eq!(sequence(ArpPattern::Octave, 1), [0]);
}

#[test]
fn zero_voices_produce_an_empty_period() {
    assert_eq!(sequence(ArpPattern::Up, 0), Vec::<usize>::new());
    assert_eq!(sequence(ArpPattern::Converge, 0), Vec::<usize>::new());
}

#[test]
fn random_has_no_period() {
    assert_eq!(ArpPattern::Random.voice_sequence(4), None);
}

#[test]
fn next_and_previous_walk_the_whole_list_and_wrap() {
    let mut pattern = ArpPattern::ALL[0];
    for expected in ArpPattern::ALL.iter().skip(1).chain(&[ArpPattern::ALL[0]]) {
        pattern = pattern.next();
        assert_eq!(pattern, *expected);
    }
    assert_eq!(pattern, ArpPattern::ALL[0]);

    for expected in ArpPattern::ALL.iter().rev() {
        assert_eq!(pattern.previous(), *expected);
        pattern = pattern.previous();
    }
}

#[test]
fn labels_are_unique() {
    let mut labels = ArpPattern::ALL.map(ArpPattern::label).to_vec();
    labels.sort_unstable();
    let count = labels.len();
    labels.dedup();
    assert_eq!(labels.len(), count);
}

#[test]
fn up_down_sequence_descends_by_the_given_width() {
    assert_eq!(up_down_sequence(6, None), [0, 1, 2, 3, 4, 5, 4, 3, 2, 1]);
    assert_eq!(up_down_sequence(6, Some(2)), [0, 1, 2, 3, 4, 5, 4, 3]);
    assert_eq!(up_down_sequence(6, Some(1)), [0, 1, 2, 3, 4, 5, 4]);
    assert_eq!(up_down_sequence(6, Some(0)), [0, 1, 2, 3, 4, 5]);
}

#[test]
fn up_down_width_is_capped_at_the_full_descent() {
    assert_eq!(up_down_sequence(6, Some(4)), up_down_sequence(6, None));
    assert_eq!(up_down_sequence(6, Some(99)), up_down_sequence(6, None));
}

#[test]
fn up_down_sequence_edges() {
    assert_eq!(up_down_sequence(0, None), Vec::<usize>::new());
    assert_eq!(up_down_sequence(0, Some(2)), Vec::<usize>::new());
    assert_eq!(up_down_sequence(1, None), [0]);
    assert_eq!(up_down_sequence(1, Some(2)), [0]);
    assert_eq!(up_down_sequence(2, None), [0, 1]);
    assert_eq!(up_down_sequence(2, Some(2)), [0, 1]);
}

#[test]
fn up_down_voice_sequence_descends_all_the_way() {
    for voice_count in 0..8 {
        assert_eq!(
            sequence(ArpPattern::UpDown, voice_count),
            up_down_sequence(voice_count, None)
        );
    }
}

#[test]
fn up_turn_is_not_a_pattern() {
    assert!(ArpPattern::ALL.iter().all(|p| p.label() != "UpTurn"));
}
