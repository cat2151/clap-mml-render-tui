use cmrt_arpeggiator::ArpPattern;

use super::*;

fn on(pattern: ArpPattern, octaves: usize) -> ArpSettings {
    ArpSettings {
        pattern,
        octaves,
        ..ArpSettings::default()
    }
}

fn notes_of(mml: &str, settings: &ArpSettings) -> Vec<Note> {
    notes_from_events(&performance_events(mml, Some(settings)).expect("MML を解釈できる"))
}

/// arp を当てない素材の音。
fn material_notes(mml: &str) -> Vec<Note> {
    notes_from_events(&performance_events(mml, None).expect("MML を解釈できる"))
}

/// 既定オクターブの `c` からの半音で書いた音高列。
fn offsets(mml: &str, settings: &ArpSettings) -> Vec<i32> {
    let c = material_notes("c")[0].pitch as i32;
    notes_of(mml, settings)
        .iter()
        .map(|note| note.pitch as i32 - c)
        .collect()
}

const C: i32 = 0;
const D: i32 = 2;
const E: i32 = 4;
const F: i32 = 5;
const G: i32 = 7;
const A: i32 = 9;
const B: i32 = 11;
const UP: i32 = 12;

#[test]
fn up_down_one_octave() {
    assert_eq!(
        offsets("l16cdef", &on(ArpPattern::UpDown, 1)),
        vec![C, D, E, F, E, D]
    );
}

#[test]
fn up_one_octave() {
    assert_eq!(offsets("l16cdef", &on(ArpPattern::Up, 1)), vec![C, D, E, F]);
}

#[test]
fn up_down_two_octaves() {
    assert_eq!(
        offsets("l16cdef", &on(ArpPattern::UpDown, 2)),
        vec![
            C,
            D,
            E,
            F,
            C + UP,
            D + UP,
            E + UP,
            F + UP,
            E + UP,
            D + UP,
            C + UP,
            F,
            E,
            D
        ]
    );
}

#[test]
fn up_down_climbs_then_descends_by_the_down_width() {
    let settings = ArpSettings {
        down: Some(2),
        ..on(ArpPattern::UpDown, 1)
    };
    let phrase = [G, A, B, C + UP, D + UP, E + UP, F + UP, E + UP, D + UP];
    assert_eq!(offsets("l16gab<cdef", &settings), phrase);
}

#[test]
fn a_down_width_past_the_range_is_clamped_to_eight() {
    let eight = ArpSettings {
        down: Some(8),
        ..on(ArpPattern::UpDown, 3)
    };
    let over = ArpSettings {
        down: Some(50),
        ..eight
    };
    // 7 声部 × 3 オクターブ = 21 声部。上り 21 + 下り 8。
    assert_eq!(offsets("l16cdefgab", &over).len(), 29);
    assert_eq!(offsets("l16cdefgab", &over), offsets("l16cdefgab", &eight));
}

#[test]
fn the_down_width_only_affects_up_down() {
    for pattern in [ArpPattern::Up, ArpPattern::Down, ArpPattern::DownUp] {
        let full = on(pattern, 2);
        let short = ArpSettings {
            down: Some(1),
            ..full
        };
        assert_eq!(
            offsets("l16cdef", &short),
            offsets("l16cdef", &full),
            "{pattern:?}"
        );
    }
}

#[test]
fn step_follows_material_spacing_and_each_note_lasts_one_step() {
    let settings = on(ArpPattern::UpDown, 1);
    let sixteenth = notes_of("l16cdef", &settings);
    let eighth = notes_of("l8cdef", &settings);
    let step16 = sixteenth[1].on_seconds - sixteenth[0].on_seconds;
    let step8 = eighth[1].on_seconds - eighth[0].on_seconds;
    assert!((step8 - 2.0 * step16).abs() < 1e-9, "{step8} vs {step16}");

    for notes in [&sixteenth, &eighth] {
        let step = notes[1].on_seconds - notes[0].on_seconds;
        for (i, note) in notes.iter().enumerate() {
            assert!((note.on_seconds - i as f64 * step).abs() < 1e-9);
            assert!((note.off_seconds - note.on_seconds - step).abs() < 1e-9);
        }
        let last_off = notes.last().unwrap().off_seconds;
        assert!((last_off - notes.len() as f64 * step).abs() < 1e-9);
    }
}

#[test]
fn single_column_uses_first_note_length_and_chord_voices() {
    let material = material_notes("'ceg'");
    assert_eq!(material.iter().map(|n| n.column).max(), Some(0));
    let length = material[0].off_seconds - material[0].on_seconds;

    let settings = on(ArpPattern::Up, 1);
    let notes = notes_of("'ceg'", &settings);
    let c = material.iter().map(|n| n.pitch).min().unwrap();
    let pitches: Vec<u8> = notes.iter().map(|n| n.pitch).collect();
    assert_eq!(pitches, vec![c, c + 4, c + 7]);
    for note in &notes {
        assert!((note.off_seconds - note.on_seconds - length).abs() < 1e-9);
    }
}

#[test]
fn without_arp_the_material_is_unchanged() {
    let plain = cmrt_chord::timed_performance("l16cdef").unwrap().events;
    assert_eq!(performance_events("l16cdef", None), Ok(plain));
}

#[test]
fn empty_mml_is_empty() {
    assert_eq!(
        performance_events("", Some(&on(ArpPattern::Up, 1))),
        Ok(Vec::new())
    );
}

#[test]
fn serde_round_trips_with_pattern_label() {
    let settings = ArpSettings {
        down: Some(3),
        ..on(ArpPattern::UpDown, 2)
    };
    let json = serde_json::to_string(&settings).unwrap();
    assert!(json.contains("\"UpDown\""), "{json}");
    assert_eq!(
        serde_json::from_str::<ArpSettings>(&json).unwrap(),
        settings
    );
}

#[test]
fn unknown_pattern_label_reads_as_up() {
    let settings: ArpSettings = serde_json::from_str(r#"{"pattern":"Sideways"}"#).unwrap();
    assert_eq!(settings.pattern, ArpPattern::Up);
}

#[test]
fn a_pattern_this_screen_does_not_offer_reads_as_up() {
    for label in ["Converge", "Diverge", "Octave", "Random"] {
        let settings: ArpSettings =
            serde_json::from_str(&format!(r#"{{"pattern":"{label}"}}"#)).unwrap();
        assert_eq!(settings.pattern, ArpPattern::Up, "{label}");
    }
}

#[test]
fn removed_patterns_read_as_up_and_the_old_turn_is_dropped() {
    for label in ["UpTurn", "UpDownHold"] {
        let settings: ArpSettings = serde_json::from_str(&format!(
            r#"{{"enabled":true,"pattern":"{label}","octaves":2,"turn":2}}"#
        ))
        .unwrap();
        assert_eq!(
            settings,
            ArpSettings {
                pattern: ArpPattern::Up,
                octaves: 2,
                down: None,
                shift: 0,
                bpm: 120,
                rate: ArpRate::Sixteenth,
            },
            "{label}"
        );
    }
}

#[test]
fn missing_fields_read_as_default() {
    assert_eq!(
        serde_json::from_str::<ArpSettings>("{}").unwrap(),
        ArpSettings::default()
    );
}

fn pitches(mml: &str, settings: &ArpSettings) -> Vec<u8> {
    notes_of(mml, settings)
        .iter()
        .map(|note| note.pitch)
        .collect()
}

#[test]
fn octave_copies_of_a_wide_material_do_not_repeat_a_pitch() {
    // `l16cdefgab<c` は 1 オクターブを超えるので、2 オクターブにすると上の c と複製の c が重なる。
    let up = pitches(crate::DEFAULT_MML, &on(ArpPattern::Up, 2));
    assert!(
        up.windows(2).all(|pair| pair[0] < pair[1]),
        "Up の 1 周期は単調増加: {up:?}"
    );
    let material = material_notes(crate::DEFAULT_MML)
        .iter()
        .map(|note| note.pitch)
        .collect::<Vec<_>>();
    let low = *material.iter().min().unwrap();
    assert_eq!(
        up.len(),
        15,
        "c〜b の 7 音 × 2 オクターブ + 最上の c: {up:?}"
    );
    assert_eq!((up[0], up[14]), (low, low + 24));
}

#[test]
fn a_chord_progression_spread_over_two_octaves_climbs_without_turning_back() {
    let up = pitches("C Am F G", &on(ArpPattern::Up, 2));
    assert!(
        up.windows(2).all(|pair| pair[0] < pair[1]),
        "Up の 1 周期は単調増加: {up:?}"
    );
}

#[test]
fn every_arpeggio_note_is_full_velocity_even_from_a_chord_material() {
    for mml in ["'ceg'", "Am7", "v5c v15<c"] {
        let material = material_notes(mml);
        let notes = notes_of(mml, &on(ArpPattern::Up, 2));
        assert!(!notes.is_empty(), "{mml}");
        assert!(
            notes.iter().all(|note| note.velocity == ARP_VELOCITY),
            "{mml}: material {:?} -> {:?}",
            material.iter().map(|n| n.velocity).collect::<Vec<_>>(),
            notes.iter().map(|n| n.velocity).collect::<Vec<_>>()
        );
    }
}

#[test]
fn shift_moves_every_voice_by_an_octave() {
    let base = on(ArpPattern::UpDown, 2);
    let lowered = ArpSettings { shift: -1, ..base };
    let raised = ArpSettings { shift: 2, ..base };
    let expected = |delta: i32| -> Vec<i32> {
        offsets("l16ceg", &base)
            .iter()
            .map(|offset| offset + delta)
            .collect()
    };
    assert_eq!(offsets("l16ceg", &lowered), expected(-12));
    assert_eq!(offsets("l16ceg", &raised), expected(24));
}

#[test]
fn shift_drops_voices_that_fall_below_midi_zero() {
    // o1 の c は MIDI 12。-2 で c は -12、e は -8 に落ち、2 オクターブ目の 0 / 4 だけ残る。
    let material = material_notes("o1l16ce")
        .iter()
        .map(|note| note.pitch)
        .collect::<Vec<_>>();
    assert_eq!(material, vec![12, 16]);
    let settings = ArpSettings {
        shift: -2,
        ..on(ArpPattern::Up, 2)
    };
    assert_eq!(pitches("o1l16ce", &settings), vec![0, 4]);
}

#[test]
fn shift_past_the_range_is_clamped() {
    let two = ArpSettings {
        shift: -2,
        ..on(ArpPattern::Up, 1)
    };
    let over = ArpSettings { shift: -9, ..two };
    assert_eq!(pitches("l16ceg", &over), pitches("l16ceg", &two));
}

/// 音の頭どうしの間隔（全部同じであることも確かめる）。
fn uniform_step(mml: &str, settings: &ArpSettings) -> f64 {
    let notes = notes_of(mml, settings);
    let step = notes[1].on_seconds - notes[0].on_seconds;
    for pair in notes.windows(2) {
        assert!(
            (pair[1].on_seconds - pair[0].on_seconds - step).abs() < 1e-9,
            "{mml} {settings:?}"
        );
    }
    for note in &notes {
        assert!((note.off_seconds - note.on_seconds - step).abs() < 1e-9);
    }
    step
}

#[test]
fn a_chord_material_steps_by_the_bpm_and_the_rate() {
    let base = on(ArpPattern::Up, 1);
    for (settings, expected) in [
        (base, 0.125),
        (ArpSettings { bpm: 90, ..base }, 60.0 / 90.0 / 4.0),
        (
            ArpSettings {
                rate: ArpRate::EighthTriplet,
                ..base
            },
            0.5 / 3.0,
        ),
        (
            ArpSettings {
                rate: ArpRate::ThirtySecond,
                ..base
            },
            0.0625,
        ),
    ] {
        let step = uniform_step("C", &settings);
        assert!((step - expected).abs() < 1e-9, "{settings:?}: {step}");
    }
    assert!((uniform_step("C", &ArpSettings { bpm: 90, ..base }) - 0.1667).abs() < 1e-4);
}

#[test]
fn a_bpm_past_the_range_is_clamped() {
    let base = on(ArpPattern::Up, 1);
    let fastest = uniform_step("C", &ArpSettings { bpm: 240, ..base });
    assert!((fastest - 0.0625).abs() < 1e-9, "{fastest}");
    assert_eq!(
        uniform_step("C", &ArpSettings { bpm: 999, ..base }),
        fastest
    );
    let slowest = uniform_step("C", &ArpSettings { bpm: 40, ..base });
    assert!((slowest - 0.375).abs() < 1e-9, "{slowest}");
    assert_eq!(uniform_step("C", &ArpSettings { bpm: 0, ..base }), slowest);
}

#[test]
fn an_mml_material_ignores_the_bpm_and_the_rate() {
    let base = on(ArpPattern::UpDown, 1);
    let plain = uniform_step("'ceg'", &base);
    let material = material_notes("'ceg'");
    assert!((plain - (material[0].off_seconds - material[0].on_seconds)).abs() < 1e-9);
    for settings in [
        ArpSettings { bpm: 60, ..base },
        ArpSettings {
            rate: ArpRate::ThirtySecond,
            ..base
        },
    ] {
        assert_eq!(uniform_step("'ceg'", &settings), plain, "{settings:?}");
    }
}

#[test]
fn material_performance_tells_whether_the_material_is_a_chord() {
    let settings = on(ArpPattern::Up, 1);
    assert!(
        material_performance("Am7", Some(&settings))
            .unwrap()
            .from_chord
    );
    assert!(
        !material_performance("'ceg'", Some(&settings))
            .unwrap()
            .from_chord
    );
    assert!(material_performance("Am7", None).unwrap().from_chord);
    assert_eq!(
        material_performance("Am7", Some(&settings)).unwrap().events,
        performance_events("Am7", Some(&settings)).unwrap()
    );
}

#[test]
fn bpm_and_rate_round_trip_and_read_as_default_when_missing() {
    let settings = ArpSettings {
        bpm: 97,
        rate: ArpRate::SixteenthTriplet,
        ..on(ArpPattern::Up, 1)
    };
    let json = serde_json::to_string(&settings).unwrap();
    assert!(json.contains("\"bpm\":97,\"rate\":\"16t\""), "{json}");
    assert_eq!(
        serde_json::from_str::<ArpSettings>(&json).unwrap(),
        settings
    );
    let old: ArpSettings =
        serde_json::from_str(r#"{"enabled":true,"pattern":"UpDown","shift":-1}"#).unwrap();
    assert_eq!((old.bpm, old.rate), (120, ArpRate::Sixteenth));
    assert_eq!(old.shift, -1);
}
