use super::*;

fn request(progressions: &[&str]) -> BassVoicingInspectRequest {
    BassVoicingInspectRequest {
        key: "Key=C".to_string(),
        progressions: progressions
            .iter()
            .map(|progression| (*progression).to_string())
            .collect(),
    }
}

#[test]
fn every_set_is_voiced_as_one_continuous_progression() {
    let request = request(&["C", "B-C"]);

    let actual = report(&request).unwrap();

    assert_eq!(
        actual,
        concat!(
            "bass-auto-voicing key=\"Key=C\" sets=2 chords=3\n",
            "set-summary set=0 degrees=\"C\" bass_note_numbers=[48] ",
            "bass_note_range=48..=48(span=0) max_jump=none\n",
            "set=0 chord=0 symbol=\"C\" source_root=60 root_pc=0 bass=48 jump=none\n",
            "set-summary set=1 degrees=\"B-C\" bass_note_numbers=[47, 48] ",
            "bass_note_range=47..=48(span=1) max_jump=1\n",
            "set=1 chord=0 symbol=\"B\" source_root=71 root_pc=11 bass=47 jump=1\n",
            "set=1 chord=1 symbol=\"C\" source_root=60 root_pc=0 bass=48 jump=1\n",
            "summary bass_note_numbers=[48, 47, 48] bass_note_range=47..=48(span=1) max_jump=1\n",
        )
    );
}

#[test]
fn saved_arrangement_report_matches_the_exact_golden_output() {
    let actual = report(&request(&[
        "I-V-VIm-IV",
        "I-II-IV-I",
        "I-VIm-IV-V",
        "I-bVII-IV-I",
        "IM7-IVM7-IIm7-V7",
        "I-IV-I",
        "Im-VII-VI-VII",
    ]))
    .unwrap();

    assert_eq!(
        actual,
        concat!(
            "bass-auto-voicing key=\"Key=C\" sets=7 chords=27\n",
            "set-summary set=0 degrees=\"I-V-VIm-IV\" bass_note_numbers=[48, 55, 57, 53] bass_note_range=48..=57(span=9) max_jump=7\n",
            "set=0 chord=0 symbol=\"I\" source_root=60 root_pc=0 bass=48 jump=none\n",
            "set=0 chord=1 symbol=\"V\" source_root=67 root_pc=7 bass=55 jump=7\n",
            "set=0 chord=2 symbol=\"VIm\" source_root=69 root_pc=9 bass=57 jump=2\n",
            "set=0 chord=3 symbol=\"IV\" source_root=65 root_pc=5 bass=53 jump=4\n",
            "set-summary set=1 degrees=\"I-II-IV-I\" bass_note_numbers=[48, 50, 53, 48] bass_note_range=48..=53(span=5) max_jump=5\n",
            "set=1 chord=0 symbol=\"I\" source_root=60 root_pc=0 bass=48 jump=5\n",
            "set=1 chord=1 symbol=\"II\" source_root=62 root_pc=2 bass=50 jump=2\n",
            "set=1 chord=2 symbol=\"IV\" source_root=65 root_pc=5 bass=53 jump=3\n",
            "set=1 chord=3 symbol=\"I\" source_root=60 root_pc=0 bass=48 jump=5\n",
            "set-summary set=2 degrees=\"I-VIm-IV-V\" bass_note_numbers=[48, 45, 41, 43] bass_note_range=41..=48(span=7) max_jump=4\n",
            "set=2 chord=0 symbol=\"I\" source_root=60 root_pc=0 bass=48 jump=0\n",
            "set=2 chord=1 symbol=\"VIm\" source_root=69 root_pc=9 bass=45 jump=3\n",
            "set=2 chord=2 symbol=\"IV\" source_root=65 root_pc=5 bass=41 jump=4\n",
            "set=2 chord=3 symbol=\"V\" source_root=67 root_pc=7 bass=43 jump=2\n",
            "set-summary set=3 degrees=\"I-bVII-IV-I\" bass_note_numbers=[48, 46, 53, 48] bass_note_range=46..=53(span=7) max_jump=7\n",
            "set=3 chord=0 symbol=\"I\" source_root=60 root_pc=0 bass=48 jump=5\n",
            "set=3 chord=1 symbol=\"bVII\" source_root=70 root_pc=10 bass=46 jump=2\n",
            "set=3 chord=2 symbol=\"IV\" source_root=65 root_pc=5 bass=53 jump=7\n",
            "set=3 chord=3 symbol=\"I\" source_root=60 root_pc=0 bass=48 jump=5\n",
            "set-summary set=4 degrees=\"IM7-IVM7-IIm7-V7\" bass_note_numbers=[48, 53, 50, 55] bass_note_range=48..=55(span=7) max_jump=5\n",
            "set=4 chord=0 symbol=\"IM7\" source_root=60 root_pc=0 bass=48 jump=0\n",
            "set=4 chord=1 symbol=\"IVM7\" source_root=65 root_pc=5 bass=53 jump=5\n",
            "set=4 chord=2 symbol=\"IIm7\" source_root=62 root_pc=2 bass=50 jump=3\n",
            "set=4 chord=3 symbol=\"V7\" source_root=67 root_pc=7 bass=55 jump=5\n",
            "set-summary set=5 degrees=\"I-IV-I\" bass_note_numbers=[48, 53, 48] bass_note_range=48..=53(span=5) max_jump=5\n",
            "set=5 chord=0 symbol=\"I\" source_root=60 root_pc=0 bass=48 jump=7\n",
            "set=5 chord=1 symbol=\"IV\" source_root=65 root_pc=5 bass=53 jump=5\n",
            "set=5 chord=2 symbol=\"I\" source_root=60 root_pc=0 bass=48 jump=5\n",
            "set-summary set=6 degrees=\"Im-VII-VI-VII\" bass_note_numbers=[48, 47, 45, 47] bass_note_range=45..=48(span=3) max_jump=2\n",
            "set=6 chord=0 symbol=\"Im\" source_root=60 root_pc=0 bass=48 jump=0\n",
            "set=6 chord=1 symbol=\"VII\" source_root=71 root_pc=11 bass=47 jump=1\n",
            "set=6 chord=2 symbol=\"VI\" source_root=69 root_pc=9 bass=45 jump=2\n",
            "set=6 chord=3 symbol=\"VII\" source_root=71 root_pc=11 bass=47 jump=2\n",
            "summary bass_note_numbers=[48, 55, 57, 53, 48, 50, 53, 48, 48, 45, 41, 43, 48, 46, 53, 48, 48, 53, 50, 55, 48, 53, 48, 48, 47, 45, 47] bass_note_range=41..=57(span=16) max_jump=7\n",
        )
    );
}

#[test]
fn a_long_set_exposes_every_octave_choice_and_the_total_span() {
    let request = request(&["I-V-VIm-IIIm-IV-I-V-IV-IIm-V-I"]);

    let actual = report(&request).unwrap();

    assert!(actual.contains("summary bass_note_numbers=["), "{actual}");
    assert!(actual.contains("bass_note_range="), "{actual}");
    assert!(actual.contains("max_jump="), "{actual}");
    assert_eq!(actual.matches("set=0 chord=").count(), 11, "{actual}");
    assert_eq!(actual.matches("set-summary set=0").count(), 1, "{actual}");
}

#[test]
fn an_invalid_set_fails_instead_of_disappearing_from_the_report() {
    let error = report(&request(&["I-V", "zzz", "V-I"]))
        .unwrap_err()
        .to_string();

    assert!(error.contains("zzz"), "{error}");
}
