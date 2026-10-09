//! pattern 1 つを、それだけで読める SMF にする。
//!
//! format 0・1 小節・[`DRUM_BPM`]・4/4・GM のドラム channel（10ch）で書き、track 名に kit の
//! patch 名を入れる。読むときは tempo・channel・track 名を見ず、拍を単位に
//! 16 分音符の step へ丸める。velocity は note on のものを打点に持たせる。

use midly::{
    num::{u15, u24, u28, u4, u7},
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
};

use crate::{DrumHit, DrumPattern, DRUM_BPM, DRUM_STEPS};

const TICKS_PER_BEAT: u16 = 480;
const STEPS_PER_BEAT: u32 = 4;
const TICKS_PER_STEP: u32 = TICKS_PER_BEAT as u32 / STEPS_PER_BEAT;
/// GM のドラム channel（10ch）。
const DRUM_CHANNEL: u8 = 9;

/// 同じ note の次の打点より長い音長は、再生と同じくその打点で切って書く。読み直すと切った
/// 長さになる。それ以外は読み直すと同じ pattern に戻る。
pub fn pattern_to_smf(kit: &str, pattern: &DrumPattern) -> Vec<u8> {
    // 同じ tick では off を先に置き、前の打点の off が次の打点の音を切らないようにする。
    // (tick, on か, note, velocity)。
    let mut notes: Vec<(u32, bool, u8, u8)> = Vec::new();
    let hits: Vec<DrumHit> = pattern.hits().collect();
    for (
        index,
        &DrumHit {
            note,
            step,
            steps,
            velocity,
        },
    ) in hits.iter().enumerate()
    {
        let next = hits
            .get(index + 1)
            .filter(|next| next.note == note)
            .map_or(usize::MAX, |next| next.step);
        let on = step as u32 * TICKS_PER_STEP;
        let off = (step + usize::from(steps)).min(next) as u32 * TICKS_PER_STEP;
        notes.push((on, true, note, velocity));
        notes.push((off, false, note, 0));
    }
    notes.sort_unstable();
    let bar_end = DRUM_STEPS as u32 * TICKS_PER_STEP;
    let end = notes
        .last()
        .map_or(bar_end, |(tick, ..)| (*tick).max(bar_end));
    let mut events = vec![
        meta(0, MetaMessage::TrackName(kit.as_bytes())),
        meta(
            0,
            MetaMessage::Tempo(u24::new((60_000_000.0 / DRUM_BPM) as u32)),
        ),
        // 4/4、metronome は 4 分音符ごと、4 分音符 = 32 分音符 8 個。
        meta(0, MetaMessage::TimeSignature(4, 2, 24, 8)),
    ];
    let mut last = 0;
    for (tick, on, note, velocity) in notes {
        let key = u7::new(note);
        let message = if on {
            MidiMessage::NoteOn {
                key,
                vel: u7::new(velocity),
            }
        } else {
            MidiMessage::NoteOff {
                key,
                vel: u7::new(0),
            }
        };
        events.push(TrackEvent {
            delta: u28::new(tick - last),
            kind: TrackEventKind::Midi {
                channel: u4::new(DRUM_CHANNEL),
                message,
            },
        });
        last = tick;
    }
    events.push(meta(end - last, MetaMessage::EndOfTrack));
    let smf = Smf {
        header: Header::new(
            Format::SingleTrack,
            Timing::Metrical(u15::new(TICKS_PER_BEAT)),
        ),
        tracks: vec![events],
    };
    let mut bytes = Vec::new();
    smf.write_std(&mut bytes)
        .expect("writing to a Vec does not fail");
    bytes
}

/// 全 track の note を 1 つの pattern にまとめる。小節の外で始まる note は捨てる。
/// off の無い note は小節の終わりまで鳴らす。
pub fn pattern_from_smf(bytes: &[u8]) -> Result<DrumPattern, String> {
    let smf = Smf::parse(bytes).map_err(|error| format!("SMF を解析できません: {error}"))?;
    let Timing::Metrical(ticks_per_beat) = smf.header.timing else {
        return Err("SMPTE 時間の SMF は読めません".to_string());
    };
    let ticks_per_beat = u64::from(ticks_per_beat.as_int().max(1));
    let to_step =
        |tick: u64| (tick * u64::from(STEPS_PER_BEAT) * 2 + ticks_per_beat) / (ticks_per_beat * 2);
    let mut hits = Vec::new();
    for track in &smf.tracks {
        let mut tick = 0_u64;
        // note ごとの、鳴っている打点の (on の tick, velocity)。
        let mut open: [Option<(u64, u8)>; 128] = [None; 128];
        let close = |note: u8, (on, velocity): (u64, u8), off: u64, hits: &mut Vec<DrumHit>| {
            let step = to_step(on);
            if step < DRUM_STEPS as u64 {
                let steps = to_step(off)
                    .saturating_sub(step)
                    .clamp(1, DRUM_STEPS as u64);
                hits.push(DrumHit {
                    note,
                    step: step as usize,
                    steps: steps as u8,
                    velocity,
                });
            }
        };
        for event in track {
            tick += u64::from(event.delta.as_int());
            let TrackEventKind::Midi { message, .. } = event.kind else {
                continue;
            };
            let (key, velocity) = match message {
                MidiMessage::NoteOn { key, vel } => (key.as_int(), vel.as_int()),
                MidiMessage::NoteOff { key, .. } => (key.as_int(), 0),
                _ => continue,
            };
            // 同じ note の on が続いたら、後の on で前の音を切る。
            if let Some(on) = open[usize::from(key)].take() {
                close(key, on, tick, &mut hits);
            }
            if velocity > 0 {
                open[usize::from(key)] = Some((tick, velocity));
            }
        }
        for (note, on) in open.iter().enumerate() {
            if let Some(on) = *on {
                let bar_end = DRUM_STEPS as u64 * ticks_per_beat / u64::from(STEPS_PER_BEAT);
                close(note as u8, on, bar_end.max(on.0), &mut hits);
            }
        }
    }
    Ok(DrumPattern::from_hits(hits))
}

fn meta(delta: u32, message: MetaMessage<'_>) -> TrackEvent<'_> {
    TrackEvent {
        delta: u28::new(delta),
        kind: TrackEventKind::Meta(message),
    }
}

#[cfg(test)]
mod tests;
