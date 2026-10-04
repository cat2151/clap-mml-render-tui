//! 列ごとの CC20 の待機・直線的な立ち上がり・維持と、同じ channel の次列への引き渡し。

use std::collections::BTreeMap;

use super::{control_change, VIBRATO_DEPTH_CC, VIBRATO_DEPTH_CC_DEFAULT};
use crate::{Note, Rule, RuleTable, TimedMidiEvent, VibratoSettings};

struct ColumnSpan {
    column: usize,
    on: f64,
    off: f64,
}

pub(super) fn events(notes: &[Note], rules: &RuleTable, end: f64) -> Vec<TimedMidiEvent> {
    let mut channels: BTreeMap<u8, BTreeMap<usize, ColumnSpan>> = BTreeMap::new();
    for note in notes {
        let span = channels
            .entry(note.channel)
            .or_default()
            .entry(note.column)
            .or_insert(ColumnSpan {
                column: note.column,
                on: note.on_seconds,
                off: note.off_seconds,
            });
        span.on = span.on.min(note.on_seconds);
        span.off = span.off.max(note.off_seconds);
    }

    let settings = rules.vibrato_settings();
    let mut out = Vec::new();
    for (channel, columns) in channels {
        let mut columns: Vec<ColumnSpan> = columns.into_values().collect();
        columns.sort_by(|a, b| a.on.total_cmp(&b.on));
        let mut previous_on = false;
        let mut used = false;
        for (i, span) in columns.iter().enumerate() {
            let on = rules.is_on(span.column, Rule::Vibrato);
            if on {
                used = true;
                let next_on = columns.get(i + 1).map_or(f64::INFINITY, |next| next.on);
                envelope(&mut out, span, next_on, channel, settings);
            } else if previous_on {
                // 前列が重なっていても、OFF 列の発音からは深さを残さない。
                out.push(depth(span.on, channel, VIBRATO_DEPTH_CC_DEFAULT));
            }
            previous_on = on;
        }
        if used {
            out.push(depth(end, channel, VIBRATO_DEPTH_CC_DEFAULT));
        }
    }
    out
}

fn envelope(
    out: &mut Vec<TimedMidiEvent>,
    span: &ColumnSpan,
    next_on: f64,
    channel: u8,
    settings: VibratoSettings,
) {
    let stop = span.off.min(next_on);
    let immediate = settings.delay_ms == 0 && settings.rise_ms == 0;
    let initial = if immediate && span.on < stop {
        settings.depth
    } else {
        VIBRATO_DEPTH_CC_DEFAULT
    };
    out.push(depth(span.on, channel, initial));

    if !immediate && settings.depth > 0 {
        if settings.rise_ms == 0 {
            let at = span.on + f64::from(settings.delay_ms) / 1000.0;
            if at < stop {
                out.push(depth(at, channel, settings.depth));
            }
        } else {
            // 7 bit の深さが 1 増える時刻だけ送る。短音に合わせて時間を圧縮しない。
            for value in 1..=settings.depth {
                let fraction = f64::from(value) / f64::from(settings.depth);
                let at = span.on
                    + (f64::from(settings.delay_ms) + f64::from(settings.rise_ms) * fraction)
                        / 1000.0;
                if at >= stop {
                    break;
                }
                out.push(depth(at, channel, value));
            }
        }
    }

    // 重なり時は次列の開始値が引き継ぐ。旧 off のリセットは作らない。
    // 同時刻の通常の連続列は、旧リセットを先に積んでから次の開始値を積む。
    if span.off <= next_on {
        out.push(depth(span.off, channel, VIBRATO_DEPTH_CC_DEFAULT));
    }
}

fn depth(seconds: f64, channel: u8, value: u8) -> TimedMidiEvent {
    control_change(seconds, channel, VIBRATO_DEPTH_CC, value)
}

#[cfg(test)]
mod tests;
