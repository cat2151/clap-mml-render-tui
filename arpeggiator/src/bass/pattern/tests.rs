use super::BassPattern;

#[test]
fn next_and_previous_walk_the_whole_list_and_wrap() {
    let mut pattern = BassPattern::ALL[0];
    for expected in BassPattern::ALL
        .iter()
        .skip(1)
        .chain(&[BassPattern::ALL[0]])
    {
        pattern = pattern.next();
        assert_eq!(pattern, *expected);
    }
    assert_eq!(pattern, BassPattern::ALL[0]);

    for expected in BassPattern::ALL.iter().rev() {
        assert_eq!(pattern.previous(), *expected);
        pattern = pattern.previous();
    }
}

#[test]
fn the_default_is_the_head_of_the_list() {
    assert_eq!(BassPattern::default(), BassPattern::ALL[0]);
}

#[test]
fn labels_are_unique() {
    let mut labels = BassPattern::ALL.map(BassPattern::label).to_vec();
    labels.sort_unstable();
    let count = labels.len();
    labels.dedup();
    assert_eq!(labels.len(), count);
}

#[test]
fn beat_rhythm_and_octave_use_match_the_documented_shapes() {
    assert_eq!(BassPattern::Whole.beat_rhythm(), None);
    assert_eq!(BassPattern::Eighth.beat_rhythm(), Some(&[2, 2][..]));
    assert_eq!(BassPattern::EighthOctave.beat_rhythm(), Some(&[2, 2][..]));
    assert_eq!(
        BassPattern::EighthTwoSixteenths.beat_rhythm(),
        Some(&[2, 1, 1][..])
    );
    assert_eq!(
        BassPattern::EighthTwoSixteenthsOctave.beat_rhythm(),
        Some(&[2, 1, 1][..])
    );
    assert_eq!(
        BassPattern::SixteenthOctave.beat_rhythm(),
        Some(&[1, 1, 1, 1][..])
    );

    assert!(!BassPattern::Whole.uses_octave());
    assert!(!BassPattern::Eighth.uses_octave());
    assert!(BassPattern::EighthOctave.uses_octave());
    assert!(!BassPattern::EighthTwoSixteenths.uses_octave());
    assert!(BassPattern::EighthTwoSixteenthsOctave.uses_octave());
    assert!(BassPattern::SixteenthOctave.uses_octave());
}

/// 刻みが1拍を過不足なく埋めることを、全 variant で担保する。
/// ここが崩れると [`super::super::generate_bass_line`] の拍の繰り返しが破綻する。
#[test]
fn every_beat_rhythm_fills_exactly_one_beat() {
    for pattern in BassPattern::ALL {
        let Some(rhythm) = pattern.beat_rhythm() else {
            continue;
        };
        assert!(
            rhythm.iter().all(|steps| *steps >= 1),
            "{}",
            pattern.label()
        );
        assert_eq!(rhythm.iter().sum::<usize>(), 4, "{}", pattern.label());
    }
}
