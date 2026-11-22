use super::*;

#[test]
fn pitch_comparison_distinguishes_vibrato_tremolo_and_silence() {
    let tone = |offset: f64, vibrato_cents: f64, tremolo: bool, harmonic: bool| {
        let mut phase = 0.0;
        (0..2 * SAMPLE_RATE)
            .flat_map(|i| {
                let modulation =
                    (std::f64::consts::TAU * 5.0 * i as f64 / SAMPLE_RATE as f64).sin();
                let frequency = 261.63 * ((offset + vibrato_cents * modulation) / 1200.0).exp2();
                phase += std::f64::consts::TAU * frequency / SAMPLE_RATE as f64;
                let gain = if tremolo { 0.6 + 0.4 * modulation } else { 1.0 };
                let overtone = if harmonic {
                    0.9 * (2.0 * phase).sin()
                } else {
                    0.0
                };
                [(gain * phase.sin() + overtone) as f32, 0.0]
            })
            .collect::<Vec<_>>()
    };
    let plain = tone(0.0, 0.0, false, false);
    let vibrato = tone(0.0, 50.0, false, false);
    let tremolo = tone(0.0, 0.0, true, false);
    let different_timbre = tone(0.0, 0.0, false, true);
    let shifted = tone(100.0, 0.0, false, true);
    let shifted_vibrato = tone(100.0, 50.0, false, true);
    let octave = tone(1200.0, 0.0, false, false);
    let silent = vec![0.0; plain.len()];
    use AudioClassification::*;
    assert_eq!(compare([&plain, &vibrato]).0, VibratoIncreaseCandidate);
    assert_eq!(compare([&vibrato, &plain]).0, VibratoDecreaseCandidate);
    assert_eq!(compare([&plain, &tremolo]).0, NoClearPitchChange);
    assert_eq!(compare([&plain, &different_timbre]).0, NoClearPitchChange);
    assert_eq!(compare([&plain, &shifted]).0, FixedPitchChangeCandidate);
    assert_eq!(compare([&shifted, &plain]).0, FixedPitchChangeCandidate);
    assert_eq!(
        compare([&plain, &shifted_vibrato]).0,
        VibratoIncreaseCandidate
    );
    assert_eq!(compare([&plain, &octave]).0, PitchUncertain);
    assert_eq!(compare([&plain, &silent]).0, PitchUncertain);
}
