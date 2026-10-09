use super::*;
use crate::DEFAULT_VELOCITY;

fn hit(note: u8, step: usize, steps: u8) -> DrumHit {
    DrumHit {
        note,
        step,
        steps,
        velocity: DEFAULT_VELOCITY,
    }
}

fn note_events(bytes: &[u8]) -> Vec<(u32, bool, u8, u8)> {
    let smf = Smf::parse(bytes).unwrap();
    let mut tick = 0;
    let mut events = Vec::new();
    for event in &smf.tracks[0] {
        tick += event.delta.as_int();
        if let TrackEventKind::Midi { channel, message } = event.kind {
            match message {
                MidiMessage::NoteOn { key, .. } => {
                    events.push((tick, true, key.as_int(), channel.as_int()))
                }
                MidiMessage::NoteOff { key, .. } => {
                    events.push((tick, false, key.as_int(), channel.as_int()))
                }
                _ => {}
            }
        }
    }
    events
}

#[test]
fn a_pattern_survives_the_round_trip_and_names_its_kit() {
    let pattern = DrumPattern::from_hits([
        hit(36, 0, 1),
        hit(36, 8, 1),
        hit(42, 2, 2),
        hit(49, 12, 16),
        hit(127, 15, 1),
        DrumHit {
            velocity: 1,
            ..hit(38, 4, 1)
        },
        DrumHit {
            velocity: 64,
            ..hit(38, 6, 1)
        },
    ]);
    let bytes = pattern_to_smf("sfz/Kits/909 キット.sfz", &pattern);
    assert_eq!(pattern_from_smf(&bytes).unwrap(), pattern);

    let smf = Smf::parse(&bytes).unwrap();
    assert_eq!(smf.header.format, Format::SingleTrack);
    let metas: Vec<_> = smf.tracks[0]
        .iter()
        .filter_map(|event| match event.kind {
            TrackEventKind::Meta(meta) => Some(meta),
            _ => None,
        })
        .collect();
    assert!(metas.contains(&MetaMessage::TrackName(
        "sfz/Kits/909 キット.sfz".as_bytes()
    )));
    assert!(metas.contains(&MetaMessage::Tempo(u24::new(500_000))));
    // 小節の外まで鳴る 49 の off の後で終わる。
    let end: u32 = smf.tracks[0].iter().map(|event| event.delta.as_int()).sum();
    assert_eq!(end, 28 * TICKS_PER_STEP);
    assert!(note_events(&bytes)
        .iter()
        .all(|(.., channel)| *channel == DRUM_CHANNEL));
}

#[test]
fn a_length_past_the_next_hit_of_the_same_note_is_cut_there_with_the_off_first() {
    let pattern = DrumPattern::from_hits([hit(46, 0, 16), hit(46, 4, 1)]);
    let bytes = pattern_to_smf("kit", &pattern);
    assert_eq!(
        note_events(&bytes),
        [
            (0, true, 46, DRUM_CHANNEL),
            (480, false, 46, DRUM_CHANNEL),
            (480, true, 46, DRUM_CHANNEL),
            (600, false, 46, DRUM_CHANNEL),
        ]
    );
    assert_eq!(
        pattern_from_smf(&bytes).unwrap(),
        DrumPattern::from_hits([hit(46, 0, 4), hit(46, 4, 1)])
    );
}

/// 他の道具が書いた SMF（別の分解能・format 1・velocity 0 の off・off 無し）も step へ丸めて読む。
/// velocity は note on のものを持たせる。
#[test]
fn foreign_files_are_quantized_to_steps_and_unclosed_notes_ring_to_the_bar_end() {
    let ppq = 96_u16;
    let midi = |delta: u32, key: u8, vel: u8| TrackEvent {
        delta: u28::new(delta),
        kind: TrackEventKind::Midi {
            channel: u4::new(0),
            message: MidiMessage::NoteOn {
                key: u7::new(key),
                vel: u7::new(vel),
            },
        },
    };
    let end = || TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    };
    let smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(ppq))),
        tracks: vec![
            // 25 tick ≒ 1 step 目、47 tick 後の velocity 0 で 2 step。
            vec![midi(25, 36, 100), midi(47, 36, 0), end()],
            // 2 小節目の打点は捨てる。off の無い 38 は小節の終わりまで。
            vec![midi(96 * 3, 38, 90), midi(96 * 2, 40, 90), end()],
        ],
    };
    let mut bytes = Vec::new();
    smf.write_std(&mut bytes).unwrap();
    assert_eq!(
        pattern_from_smf(&bytes).unwrap(),
        DrumPattern::from_hits([
            DrumHit {
                velocity: 100,
                ..hit(36, 1, 2)
            },
            DrumHit {
                velocity: 90,
                ..hit(38, 12, 4)
            },
        ])
    );
    assert!(pattern_from_smf(b"not a midi file").is_err());
}
