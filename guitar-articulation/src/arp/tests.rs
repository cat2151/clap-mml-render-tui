use cmrt_arpeggiator::ArpPattern;

use super::*;

fn on(pattern: ArpPattern, octaves: usize, cycles: usize) -> ArpSettings {
    ArpSettings {
        enabled: true,
        pattern,
        octaves,
        cycles,
        ..ArpSettings::default()
    }
}

fn notes_of(mml: &str, settings: &ArpSettings) -> Vec<Note> {
    notes_from_events(&performance_events(mml, settings).expect("MML を解釈できる"))
}

/// 既定オクターブの `c` からの半音で書いた音高列。
fn offsets(mml: &str, settings: &ArpSettings) -> Vec<i32> {
    let c = notes_of("c", &ArpSettings::default())[0].pitch as i32;
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
fn up_down_one_octave_two_cycles() {
    assert_eq!(
        offsets("l16cdef", &on(ArpPattern::UpDown, 1, 2)),
        vec![C, D, E, F, E, D, C, D, E, F, E, D]
    );
}

#[test]
fn up_one_octave_two_cycles() {
    assert_eq!(
        offsets("l16cdef", &on(ArpPattern::Up, 1, 2)),
        vec![C, D, E, F, C, D, E, F]
    );
}

#[test]
fn up_down_two_octaves_one_cycle() {
    assert_eq!(
        offsets("l16cdef", &on(ArpPattern::UpDown, 2, 1)),
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
fn up_turn_climbs_then_turns_back_by_two() {
    let settings = ArpSettings {
        turn: 2,
        ..on(ArpPattern::UpTurn, 1, 2)
    };
    let phrase = [G, A, B, C + UP, D + UP, E + UP, F + UP, E + UP, D + UP];
    assert_eq!(offsets("l16gab<cdef", &settings), [phrase, phrase].concat());
}

#[test]
fn step_follows_material_spacing_and_each_note_lasts_one_step() {
    let settings = on(ArpPattern::UpDown, 1, 2);
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
    let material = notes_of("'ceg'", &ArpSettings::default());
    assert_eq!(material.iter().map(|n| n.column).max(), Some(0));
    let length = material[0].off_seconds - material[0].on_seconds;

    let settings = on(ArpPattern::Up, 1, 1);
    let notes = notes_of("'ceg'", &settings);
    let c = material.iter().map(|n| n.pitch).min().unwrap();
    let pitches: Vec<u8> = notes.iter().map(|n| n.pitch).collect();
    assert_eq!(pitches, vec![c, c + 4, c + 7]);
    for note in &notes {
        assert!((note.off_seconds - note.on_seconds - length).abs() < 1e-9);
    }
}

#[test]
fn disabled_returns_material_unchanged() {
    let plain = cmrt_chord::timed_performance("l16cdef").unwrap().events;
    let settings = ArpSettings {
        enabled: false,
        ..on(ArpPattern::UpDown, 3, 8)
    };
    assert_eq!(arpeggiate(&plain, &settings), plain);
}

#[test]
fn empty_mml_is_empty() {
    assert_eq!(
        performance_events("", &on(ArpPattern::Up, 1, 2)),
        Ok(Vec::new())
    );
}

#[test]
fn serde_round_trips_with_pattern_label() {
    let settings = ArpSettings {
        turn: 3,
        ..on(ArpPattern::UpTurn, 2, 5)
    };
    let json = serde_json::to_string(&settings).unwrap();
    assert!(json.contains("\"UpTurn\""), "{json}");
    assert_eq!(
        serde_json::from_str::<ArpSettings>(&json).unwrap(),
        settings
    );
}

#[test]
fn unknown_pattern_label_reads_as_up() {
    let settings: ArpSettings =
        serde_json::from_str(r#"{"enabled":true,"pattern":"Sideways"}"#).unwrap();
    assert_eq!(settings.pattern, ArpPattern::Up);
    assert!(settings.enabled);
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
fn missing_fields_read_as_default() {
    assert_eq!(
        serde_json::from_str::<ArpSettings>("{}").unwrap(),
        ArpSettings::default()
    );
}
