use super::*;

fn context(progressions: &[&str], selected: usize) -> PreviewVoicingContext {
    PreviewVoicingContext {
        progressions: progressions
            .iter()
            .map(|progression| (*progression).to_string())
            .collect(),
        selected,
    }
}

#[test]
fn a_section_is_voiced_inside_its_own_progression() {
    let context = context(&["I-IV-V-I"], 0);
    let actual = selected_voicings(Some("Key:C"), &context).unwrap();
    let parsed = cmrt_chord::parse_chord_progression("Key:C I-IV-V-I").unwrap();

    assert_eq!(
        actual,
        cmrt_chord::auto_voice_with_key(parsed.chords(), parsed.key_pitch_class(), None)
    );
}

#[test]
fn an_arrangement_is_voiced_as_one_continuous_progression() {
    let context = context(&["I-IV", "V-I", "VIm-IV"], 1);
    let actual = selected_voicings(Some("Key:C"), &context).unwrap();
    let all = cmrt_chord::parse_chord_progression("Key:C I-IV-V-I-VIm-IV").unwrap();
    let expected = cmrt_chord::auto_voice_with_key(all.chords(), all.key_pitch_class(), None);

    assert_eq!(actual, expected[2..4]);
}

#[test]
fn an_unreadable_section_breaks_the_chain_without_silencing_its_neighbor() {
    let sections = ["I-IV", "zzz", "V-I"];
    for (selected, degrees) in [(0, "I-IV"), (2, "V-I")] {
        let actual = selected_voicings(Some("Key:C"), &context(&sections, selected)).unwrap();
        let parsed = cmrt_chord::parse_chord_progression(&format!("Key:C {degrees}")).unwrap();
        assert_eq!(
            actual,
            cmrt_chord::auto_voice_with_key(parsed.chords(), parsed.key_pitch_class(), None),
            "valid section {selected} was not voiced as its own run"
        );
    }
    assert!(selected_voicings(Some("Key:C"), &context(&sections, 1)).is_none());
}

#[test]
fn saved_arrangement_matches_the_golden_bass_path() {
    let sections = [
        "I-V-VIm-IV",
        "I-II-IV-I",
        "I-VIm-IV-V",
        "I-bVII-IV-I",
        "IM7-IVM7-IIm7-V7",
        "I-IV-I",
        "Im-VII-VI-VII",
    ];
    let arrangement = context(&sections, 0);
    let basses = (0..sections.len())
        .flat_map(|selected| {
            let mut selected_context = arrangement.clone();
            selected_context.selected = selected;
            selected_voicings(Some("Key:C"), &selected_context)
                .unwrap()
                .into_iter()
                .map(|voicing| voicing.bass.unwrap())
        })
        .collect::<Vec<_>>();

    assert_eq!(
        basses,
        [
            48, 55, 57, 53, 48, 50, 53, 48, 48, 45, 41, 43, 48, 46, 53, 48, 48, 53, 50, 55, 48, 53,
            48, 48, 47, 45, 47,
        ]
    );
    assert_eq!(
        basses.iter().max().unwrap() - basses.iter().min().unwrap(),
        16
    );
    assert_eq!(
        basses
            .windows(2)
            .map(|pair| pair[0].abs_diff(pair[1]))
            .max(),
        Some(7)
    );
}
