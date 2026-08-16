//! コード表記または MML を、そのまま演奏できる「時刻つき MIDI イベント列」へ変換する。
//!
//! [`crate::note_progression`] は同時発音のまとまりだけを返し、いつ鳴っていつ止むかを
//! 捨てている（鍵盤のように1和音ずつ手で送る用途のため）。フレーズを書いたとおりの
//! 音長で鳴らしたい呼び出し側はこちらを使う。
//!
//! 秒は SMF の tick と tempo メタイベントから求める。テンポは MML の `t` で
//! 途中変更できるので、単一の BPM ではなく tempo map として畳む。

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;

/// SMF に tempo 指定が無いときの既定（MIDI の規定値 = 120 BPM）。
const DEFAULT_MICROS_PER_BEAT: f64 = 500_000.0;

const MICROS_PER_SECOND: f64 = 1_000_000.0;

/// フレーズ先頭を 0 秒とした MIDI イベント。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedMidiEvent {
    pub seconds: f64,
    pub message: [u8; 3],
}

/// 1 フレーズぶんの演奏内容。
#[derive(Clone, Debug, PartialEq)]
pub struct TimedPerformance {
    /// 時刻順に並んだ note on / note off。同時刻では note off が先に来る。
    pub events: Vec<TimedMidiEvent>,
    /// 最後のイベントまでの長さ。
    pub duration_seconds: f64,
    /// 入力が chord 表記として解釈されたか。MML として素通ししたなら false。
    pub from_chord: bool,
}

/// chord2mml を通した結果。
pub struct ResolvedMml {
    pub mml: String,
    /// chord2mml が受け付けたか。受け付けなければ入力をそのまま MML として扱う。
    pub from_chord: bool,
}

/// コード表記なら MML へ直し、そうでなければ入力をそのまま返す。
///
/// chord2mml が受け付けない表記は `Err` で返ってくるので、それを「MML だった」の
/// 判定に使う。`ceg` のような MML は `Err` になり素通しされるが、`C` は
/// コードとして解釈される。どちらだったかは呼び出し側が表示できるよう返す。
pub fn resolve_chord_or_mml(input: &str) -> ResolvedMml {
    match chord2mml_core::convert(input) {
        Ok(mml) => ResolvedMml {
            mml,
            from_chord: true,
        },
        Err(_) => ResolvedMml {
            mml: input.to_string(),
            from_chord: false,
        },
    }
}

/// コード表記または MML を、時刻つきイベント列へ変換する。
pub fn timed_performance(input: &str) -> Result<TimedPerformance, String> {
    if input.trim().is_empty() {
        return Err("MMLを入力してください".to_string());
    }
    let resolved = resolve_chord_or_mml(input);
    let bytes = mmlabc_to_smf::mml_to_smf_bytes(&resolved.mml)
        .map_err(|error| format!("MML変換に失敗しました: {error}"))?;
    let (events, duration_seconds) = timed_events_from_smf(&bytes)?;
    Ok(TimedPerformance {
        events,
        duration_seconds,
        from_chord: resolved.from_chord,
    })
}

/// tempo map の折れ点。`(tick, その tick の秒, そこからの micros per beat)`。
type TempoPoint = (u64, f64, f64);

fn timed_events_from_smf(bytes: &[u8]) -> Result<(Vec<TimedMidiEvent>, f64), String> {
    let smf = Smf::parse(bytes).map_err(|error| format!("SMF解析に失敗しました: {error}"))?;
    let ticks_per_beat = match smf.header.timing {
        Timing::Metrical(ticks) => f64::from(ticks.as_int()),
        Timing::Timecode(..) => return Err("SMFのtimecode形式には未対応です".to_string()),
    };
    if ticks_per_beat <= 0.0 {
        return Err("SMFの分解能が0です".to_string());
    }

    // tempo はトラックを跨いで効くので、先に全トラックから集めてから秒へ直す。
    let mut tempo_changes = Vec::new();
    let mut messages = Vec::new();
    let mut last_tick = 0;
    for (track_index, track) in smf.tracks.iter().enumerate() {
        let mut tick = 0_u64;
        for (event_index, event) in track.iter().enumerate() {
            tick = tick.saturating_add(u64::from(event.delta.as_int()));
            last_tick = last_tick.max(tick);
            match event.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(micros)) => {
                    tempo_changes.push((tick, f64::from(micros.as_int())));
                }
                TrackEventKind::Midi { message, .. } => {
                    if let Some(message) = midi_bytes(message) {
                        messages.push((tick, message, track_index, event_index));
                    }
                }
                _ => {}
            }
        }
    }
    if messages.is_empty() {
        return Err("MMLに発音ノートがありません".to_string());
    }

    // 同じ時刻では note off を先に出す。同じ音高を続けて鳴らすとき、後から届いた
    // note off が新しい note on を消してしまうのを防ぐ。
    messages.sort_by_key(|&(tick, message, track_index, event_index)| {
        (tick, message[0] == NOTE_ON, track_index, event_index)
    });

    let tempo_map = build_tempo_map(tempo_changes, ticks_per_beat);
    let events = messages
        .into_iter()
        .map(|(tick, message, _, _)| TimedMidiEvent {
            seconds: tick_seconds(&tempo_map, tick, ticks_per_beat),
            message,
        })
        .collect();
    Ok((events, tick_seconds(&tempo_map, last_tick, ticks_per_beat)))
}

/// velocity 0 の note on は note off として扱う（SMF の慣習）。
/// program change などフレーズの発音に関わらないメッセージは落とす。
fn midi_bytes(message: MidiMessage) -> Option<[u8; 3]> {
    match message {
        MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
            Some([NOTE_ON, key.as_int(), vel.as_int()])
        }
        MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
            Some([NOTE_OFF, key.as_int(), 0])
        }
        _ => None,
    }
}

fn build_tempo_map(mut changes: Vec<(u64, f64)>, ticks_per_beat: f64) -> Vec<TempoPoint> {
    changes.sort_by_key(|&(tick, _)| tick);
    let mut points: Vec<TempoPoint> = vec![(0, 0.0, DEFAULT_MICROS_PER_BEAT)];
    for (tick, micros_per_beat) in changes {
        let &(last_tick, last_seconds, last_micros) =
            points.last().expect("tempo map always starts at tick 0");
        if tick <= last_tick {
            // 同じ tick に複数あるときは後勝ち。原点の既定値もこれで上書きされる。
            points
                .last_mut()
                .expect("tempo map always starts at tick 0")
                .2 = micros_per_beat;
            continue;
        }
        let seconds =
            last_seconds + tick_span_seconds(tick - last_tick, last_micros, ticks_per_beat);
        points.push((tick, seconds, micros_per_beat));
    }
    points
}

fn tick_span_seconds(ticks: u64, micros_per_beat: f64, ticks_per_beat: f64) -> f64 {
    ticks as f64 * micros_per_beat / ticks_per_beat / MICROS_PER_SECOND
}

fn tick_seconds(points: &[TempoPoint], tick: u64, ticks_per_beat: f64) -> f64 {
    let index = points
        .partition_point(|&(point_tick, _, _)| point_tick <= tick)
        .saturating_sub(1);
    let (base_tick, base_seconds, micros_per_beat) = points[index];
    base_seconds + tick_span_seconds(tick - base_tick, micros_per_beat, ticks_per_beat)
}

#[cfg(test)]
mod tests;
