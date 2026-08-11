use super::{generate_bass_line, BassPattern, BASS_OCTAVE_VOICE, BASS_ROOT_VOICE};

const STEPS: usize = 16;

/// (step, voice, duration_steps) の並びへ潰して、譜面をそのまま読めるようにする。
fn line(pattern: BassPattern, steps: usize) -> Vec<(usize, usize, usize)> {
    generate_bass_line(pattern, steps)
        .into_iter()
        .map(|note| (note.step, note.voice, note.duration_steps))
        .collect()
}

#[test]
fn a_whole_note_fills_the_bar_with_the_root_alone() {
    assert_eq!(line(BassPattern::Whole, STEPS), [(0, BASS_ROOT_VOICE, 16)]);
}

#[test]
fn eighths_place_eight_root_notes_of_two_steps() {
    assert_eq!(
        line(BassPattern::Eighth, STEPS),
        [
            (0, 0, 2),
            (2, 0, 2),
            (4, 0, 2),
            (6, 0, 2),
            (8, 0, 2),
            (10, 0, 2),
            (12, 0, 2),
            (14, 0, 2),
        ]
    );
}

#[test]
fn octave_eighths_alternate_every_note() {
    assert_eq!(
        line(BassPattern::EighthOctave, STEPS),
        [
            (0, 0, 2),
            (2, 1, 2),
            (4, 0, 2),
            (6, 1, 2),
            (8, 0, 2),
            (10, 1, 2),
            (12, 0, 2),
            (14, 1, 2),
        ]
    );
}

#[test]
fn the_eighth_two_sixteenths_pattern_repeats_every_beat() {
    // 1拍が「八分・16分・16分」。4拍ぶん繰り返す。
    assert_eq!(
        line(BassPattern::EighthTwoSixteenths, STEPS),
        [
            (0, 0, 2),
            (2, 0, 1),
            (3, 0, 1),
            (4, 0, 2),
            (6, 0, 1),
            (7, 0, 1),
            (8, 0, 2),
            (10, 0, 1),
            (11, 0, 1),
            (12, 0, 2),
            (14, 0, 1),
            (15, 0, 1),
        ]
    );
}

#[test]
fn the_octave_version_puts_both_sixteenths_an_octave_up() {
    // 八分音符ごとの切替なので、拍あたり root(八分) → octave(16分) → octave(16分)。
    let voices = generate_bass_line(BassPattern::EighthTwoSixteenthsOctave, STEPS)
        .into_iter()
        .map(|note| note.voice)
        .collect::<Vec<_>>();
    assert_eq!(voices, [0, 1, 1].repeat(4));
}

#[test]
fn sixteenths_switch_octave_every_two_notes() {
    // 切替は八分単位のままなので、拍あたり root・root・octave・octave。
    assert_eq!(
        line(BassPattern::SixteenthOctave, STEPS),
        [
            (0, 0, 1),
            (1, 0, 1),
            (2, 1, 1),
            (3, 1, 1),
            (4, 0, 1),
            (5, 0, 1),
            (6, 1, 1),
            (7, 1, 1),
            (8, 0, 1),
            (9, 0, 1),
            (10, 1, 1),
            (11, 1, 1),
            (12, 0, 1),
            (13, 0, 1),
            (14, 1, 1),
            (15, 1, 1),
        ]
    );
}

#[test]
fn only_the_octave_patterns_reach_the_upper_voice() {
    for pattern in BassPattern::ALL {
        let reaches_octave = generate_bass_line(pattern, STEPS)
            .iter()
            .any(|note| note.voice == BASS_OCTAVE_VOICE);
        assert_eq!(
            reaches_octave,
            matches!(
                pattern,
                BassPattern::EighthOctave
                    | BassPattern::EighthTwoSixteenthsOctave
                    | BassPattern::SixteenthOctave
            ),
            "{}",
            pattern.label()
        );
    }
}

#[test]
fn every_pattern_covers_the_bar_without_running_past_it() {
    for pattern in BassPattern::ALL {
        let notes = generate_bass_line(pattern, STEPS);
        assert!(!notes.is_empty(), "{}", pattern.label());
        for note in &notes {
            assert!(note.duration_steps >= 1, "{}", pattern.label());
            assert!(
                note.step + note.duration_steps <= STEPS,
                "{} ran past the bar",
                pattern.label()
            );
        }
        // 隙間なく敷き詰める（休符入りはまだ持たない）。
        let covered = notes.iter().map(|note| note.duration_steps).sum::<usize>();
        assert_eq!(covered, STEPS, "{}", pattern.label());
    }
}

#[test]
fn a_bar_that_does_not_divide_evenly_truncates_the_last_note() {
    assert_eq!(
        line(BassPattern::Eighth, 5),
        [(0, 0, 2), (2, 0, 2), (4, 0, 1)]
    );
    // 拍の途中で切れる場合も、次の拍の頭で打ち切って伸ばしすぎない。
    assert_eq!(
        line(BassPattern::EighthTwoSixteenths, 5),
        [(0, 0, 2), (2, 0, 1), (3, 0, 1), (4, 0, 1)]
    );
    assert_eq!(line(BassPattern::Whole, 5), [(0, 0, 5)]);
}

#[test]
fn an_empty_bar_produces_nothing() {
    for pattern in BassPattern::ALL {
        assert!(
            generate_bass_line(pattern, 0).is_empty(),
            "{}",
            pattern.label()
        );
    }
}
