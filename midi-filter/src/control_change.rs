//! LFO を CC（control change）イベント列にして、既存の演奏へ差し込む。

use crate::{sort_for_playback, Span, TimedMidiEvent, TriangleLfo};

#[cfg(test)]
mod tests;

/// modulation wheel。CC 番号は MIDI 規格で固定。
pub const MODULATION_CC: u8 = 1;

/// channel 0 の control change。overlay の演奏は 1 instance = 1 channel 0 で送る。
const CONTROL_CHANGE_STATUS: u8 = 0xB0;

/// `span` の範囲に、LFO の値が変わる点だけ CC を差し込む。
///
/// 差し込んだ後に [`sort_for_playback`] を通すので、同時刻の note on より必ず前に来る。
/// `events` に元から入っていた同時刻・同種のイベントの相対順は変わらない。
pub fn insert_control_change(
    events: &mut Vec<TimedMidiEvent>,
    controller: u8,
    lfo: &TriangleLfo,
    span: Span,
) {
    for (seconds, value) in lfo.change_points(span) {
        events.push(TimedMidiEvent {
            seconds,
            message: [CONTROL_CHANGE_STATUS, controller & 0x7F, value & 0x7F],
        });
    }
    sort_for_playback(events);
}
