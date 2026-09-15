use super::*;

#[test]
fn chord_chart_degrees_are_split_without_reinterpreting_the_source() {
    assert_eq!(
        split_chords("I-V-VIm-IV").unwrap(),
        vec!["I", "V", "VIm", "IV"]
    );
}

#[test]
fn the_first_program_is_the_whole_line_and_the_rest_are_single_chords() {
    let chords = split_chords("I-V-VIm-IV").unwrap();
    let programs = chord_programs("Key=C", "I-V-VIm-IV", &chords).unwrap();

    assert_eq!(programs[0].0, "I-V-VIm-IV");
    assert_eq!(programs[0].1.events().len(), 24);
    assert_eq!(programs[1].0, "V");
    assert_eq!(programs[1].1.events().len(), 6);
    assert_eq!(programs[2].0, "VIm");
    assert_eq!(programs[2].1.events().len(), 6);
    assert_eq!(programs[3].0, "IV");
    assert_eq!(programs[3].1.events().len(), 6);
}

#[test]
fn fixed_duration_segment_analysis_ignores_each_boundary() {
    let chords = vec!["I".to_string(), "V".to_string()];
    let mut samples = vec![0.0; 400];
    samples[50..150].fill(0.25);
    samples[250..350].fill(0.5);

    let results = analyze_segments(&samples, 2, 100, 1_000, &chords);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].peak, 0.25);
    assert_eq!(results[1].peak, 0.5);
    assert!(!results[0].silent);
    assert!(!results[1].silent);
}
