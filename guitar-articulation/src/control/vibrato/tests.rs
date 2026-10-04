use super::*;
use crate::{convert, KEYSWITCH_VELOCITY, VIBRATO_DEPTH};

fn note(column: usize, on_seconds: f64, off_seconds: f64) -> Note {
    Note {
        column,
        on_seconds,
        off_seconds,
        channel: 0,
        pitch: 52 + column as u8,
        velocity: 100,
    }
}

fn table(columns: &[usize], settings: VibratoSettings) -> RuleTable {
    let mut rules = RuleTable::default();
    for &column in columns {
        rules.toggle(column, Rule::Vibrato);
    }
    rules.set_vibrato_settings(settings);
    rules
}

fn values(out: &[TimedMidiEvent]) -> Vec<(f64, u8)> {
    out.iter()
        .filter(|e| e.message[0] & 0xF0 == 0xB0 && e.message[1] == VIBRATO_DEPTH_CC)
        .map(|e| (e.seconds, e.message[2]))
        .collect()
}

fn controls(notes: &[Note], rules: &RuleTable) -> Vec<(f64, u8)> {
    let articulations = vec![crate::Articulation::SusDown; notes.len()];
    let mut out = super::super::control_events(notes, &articulations, rules);
    cmrt_midi_filter::sort_for_playback(&mut out);
    values(&out)
}

fn at(out: &[(f64, u8)], seconds: f64) -> u8 {
    out.iter().rfind(|(t, _)| *t <= seconds).unwrap().1
}

#[test]
fn long_notes_wait_rise_linearly_hold_and_reset() {
    let settings = VibratoSettings {
        depth: 96,
        ..VibratoSettings::default()
    };
    let out = controls(&[note(0, 0.0, 2.0)], &table(&[0], settings));
    assert_eq!(out.first(), Some(&(0.0, 0)));
    assert!(out.iter().filter(|(t, _)| *t <= 0.3).all(|(_, v)| *v == 0));
    let rising: Vec<_> = out.iter().filter(|(t, _)| *t > 0.3 && *t < 0.7).collect();
    assert!(rising.iter().any(|(_, v)| *v > 0 && *v < settings.depth));
    assert!(rising.windows(2).all(|w| w[0].1 < w[1].1));
    assert!((i16::from(at(&out, 0.5)) - 48).abs() <= 1);
    assert!(out.contains(&(0.7, settings.depth)));
    assert_eq!(at(&out, 0.8), settings.depth);
    assert_eq!(at(&out, 1.99), settings.depth);
    assert_eq!(out.last(), Some(&(2.0, 0)));
}

#[test]
fn short_notes_do_not_compress_wait_or_rise_and_end_wins_at_arrival() {
    for off in [0.2, 0.5, 0.7] {
        let out = controls(
            &[note(0, 0.0, off)],
            &table(&[0], VibratoSettings::default()),
        );
        assert!(out.iter().all(|(t, v)| *v == 0 || *t < off));
        assert!(out.iter().all(|(_, v)| *v < VIBRATO_DEPTH));
        assert_eq!(at(&out, off), 0);
        if off == 0.2 {
            assert!(out.iter().all(|(_, v)| *v == 0));
        } else {
            assert!(out.iter().any(|(_, v)| *v > 0));
        }
    }
}

#[test]
fn zero_delay_rise_and_depth_keep_their_independent_meanings() {
    let out = controls(
        &[note(0, 0.0, 2.0)],
        &table(
            &[0],
            VibratoSettings {
                delay_ms: 0,
                rise_ms: 400,
                depth: 64,
            },
        ),
    );
    assert_eq!(out[0], (0.0, 0));
    assert!(out
        .iter()
        .any(|(t, v)| *t > 0.0 && *t < 0.3 && *v > 0 && *v < 64));
    assert!(out.contains(&(0.4, 64)));

    let settings = VibratoSettings {
        delay_ms: 300,
        rise_ms: 0,
        depth: 96,
    };
    let out = controls(&[note(0, 0.0, 2.0)], &table(&[0], settings));
    assert_eq!(at(&out, 0.299), 0);
    assert_eq!(at(&out, 0.3), 96);
    let out = controls(&[note(0, 0.0, 0.3)], &table(&[0], settings));
    assert!(out.iter().all(|(_, v)| *v == 0));

    let immediate = VibratoSettings {
        delay_ms: 0,
        rise_ms: 0,
        depth: 96,
    };
    let out = controls(&[note(0, 0.0, 2.0)], &table(&[0], immediate));
    assert_eq!(
        out.iter().filter(|(t, _)| *t == 0.0).collect::<Vec<_>>(),
        vec![&(0.0, 96)]
    );
    assert_eq!(at(&out, 1.99), 96);
    // 長さ 0 では即時設定でも終了を優先する。
    let out = controls(&[note(0, 0.0, 0.0)], &table(&[0], immediate));
    assert!(out.iter().all(|(_, v)| *v == 0));

    let out = controls(
        &[note(0, 0.0, 2.0)],
        &table(
            &[0],
            VibratoSettings {
                depth: 0,
                ..VibratoSettings::default()
            },
        ),
    );
    assert!(out.iter().all(|(_, v)| *v == 0));
}

#[test]
fn chords_have_one_envelope_from_earliest_on_to_last_off() {
    let rules = table(&[0], VibratoSettings::default());
    let chord = [note(0, 0.02, 1.8), note(0, 0.01, 2.0), note(0, 0.03, 1.9)];
    let out = controls(&chord, &rules);
    assert_eq!(out, controls(&[note(0, 0.01, 2.0)], &rules));
    assert_eq!(out.first(), Some(&(0.01, 0)));
    assert_eq!(out.last(), Some(&(2.0, 0)));
}

#[test]
fn an_off_column_cancels_the_previous_wait_or_rise_during_overlap() {
    for (delay_ms, next_on) in [(1000, 0.5), (0, 0.2)] {
        let settings = VibratoSettings {
            delay_ms,
            rise_ms: 400,
            depth: 64,
        };
        let out = controls(
            &[note(0, 0.0, 2.0), note(1, next_on, 3.0)],
            &table(&[0], settings),
        );
        assert_eq!(at(&out, next_on), 0);
        assert!(out.iter().all(|(t, v)| *t < next_on || *v == 0));
        assert!(!out.iter().any(|(t, _)| *t == 2.0));
    }
}

#[test]
fn a_later_on_column_restarts_wait_and_owns_the_depth_past_the_old_off() {
    let settings = VibratoSettings {
        delay_ms: 1000,
        rise_ms: 400,
        depth: 64,
    };
    let out = controls(
        &[note(0, 0.0, 2.0), note(1, 0.5, 3.0)],
        &table(&[0, 1], settings),
    );
    assert_eq!(at(&out, 0.5), 0);
    assert!(out.iter().filter(|(t, _)| *t <= 1.5).all(|(_, v)| *v == 0));
    assert!(out
        .iter()
        .any(|(t, v)| *t > 1.5 && *t < 1.9 && *v > 0 && *v < 64));
    assert!(out.contains(&(1.9, 64)));
    assert_eq!(at(&out, 2.1), 64);
    assert!(!out.iter().any(|(t, _)| *t == 2.0));
    assert_eq!(out.last(), Some(&(3.0, 0)));
}

#[test]
fn adjacent_immediate_columns_reset_before_the_next_start_value() {
    let settings = VibratoSettings {
        delay_ms: 0,
        rise_ms: 0,
        depth: 96,
    };
    let rules = table(&[0, 1], settings);
    let out = controls(&[note(0, 0.0, 0.5), note(1, 0.5, 1.0)], &rules);
    assert_eq!(
        out.iter()
            .filter(|(t, _)| *t == 0.5)
            .map(|(_, v)| *v)
            .collect::<Vec<_>>(),
        vec![0, 96]
    );
    assert_eq!(at(&out, 0.75), 96);
    let overlap = controls(&[note(0, 0.0, 0.75), note(1, 0.5, 1.0)], &rules);
    assert!(!overlap.iter().any(|(t, _)| *t == 0.75));
    assert_eq!(at(&overlap, 0.8), 96);
}

#[test]
fn no_vibrato_has_no_generated_cc20() {
    assert!(controls(&[note(0, 0.0, 2.0)], &RuleTable::default()).is_empty());
}

#[test]
fn vibrato_stacks_with_palm_mute_and_short_columns_stay_zero() {
    let mut rules = table(&[1], VibratoSettings::default());
    rules.toggle(1, Rule::PalmMute);
    assert!(rules.is_on(1, Rule::PalmMute) && rules.is_on(1, Rule::Vibrato));
    let raw = cmrt_chord::timed_performance("o3 l8 e g a").unwrap().events;
    let out = convert(&raw, &rules);
    assert!(out
        .iter()
        .any(|e| e.seconds == 0.25 && e.message == [0x90, 20, KEYSWITCH_VELOCITY]));
    assert!(values(&out).iter().all(|(_, v)| *v == 0));
    assert_eq!(values(&out).last(), Some(&(0.75, 0)));
}

#[test]
fn immediate_start_and_end_cc_precede_the_played_notes() {
    let rules = table(
        &[1],
        VibratoSettings {
            delay_ms: 0,
            rise_ms: 0,
            depth: 64,
        },
    );
    let raw = cmrt_chord::timed_performance("o3 l8 e g a").unwrap().events;
    let out = convert(&raw, &rules);
    for (seconds, pitch, depth) in [(0.25, 43, 64), (0.5, 45, 0)] {
        let at: Vec<_> = out
            .iter()
            .filter(|e| e.seconds == seconds)
            .map(|e| e.message)
            .collect();
        let cc = at
            .iter()
            .position(|m| *m == [0xB0, VIBRATO_DEPTH_CC, depth])
            .unwrap();
        let on = at.iter().position(|m| *m == [0x90, pitch, 127]).unwrap();
        assert!(cc < on);
    }
    let head: Vec<_> = values(&out)
        .into_iter()
        .filter(|(t, _)| *t == 0.0)
        .collect();
    assert_eq!(head, vec![(0.0, 0)]);
}

#[test]
fn vibrato_rule_round_trips_through_json() {
    let mut rules = table(&[1], VibratoSettings::default());
    rules.toggle(1, Rule::Slide);
    assert_eq!(RuleTable::from_json(&rules.to_json()).unwrap(), rules);
}
