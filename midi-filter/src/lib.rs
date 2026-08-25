//! 「時刻つき MIDI イベント列」を別の列へ変換する、依存なしの純粋なライブラリ。
//!
//! ウォールクロックも I/O も持たない。入力も出力も **フレーズ先頭を 0 秒とした絶対秒**で、
//! いつ実際に鳴らすか（先読み・スケジューリング）は呼び出し側の責務。
//!
//! 用意してあるのは 4 つだけ:
//!
//! - [`TriangleLfo`] … `min → max → min` を 1 周する三角波。**値が変わる点だけ**を列挙する
//! - [`insert_control_change`] … LFO を CC イベント列にして差し込む（modulation は [`MODULATION_CC`]）
//! - [`override_note_velocity`] … note on の velocity を、その音自身の時刻の LFO 値で乗っ取る
//! - [`shift`] … 1 周ぶんを k 周目の絶対秒へずらす（repeat 用）
//!
//! ```
//! use cmrt_midi_filter::{
//!     insert_control_change, shift, Span, TimedMidiEvent, TriangleLfo, MODULATION_CC,
//! };
//!
//! let cycle = vec![TimedMidiEvent { seconds: 0.0, message: [0x90, 60, 100] }];
//! // 2 周目は 1.5 秒ずらす。
//! let mut events = shift(&cycle, 1.5);
//! assert_eq!(events[0].seconds, 1.5);
//!
//! let lfo = TriangleLfo::new(4.0, 0, 127);
//! insert_control_change(&mut events, MODULATION_CC, &lfo, Span::new(1.5, 3.0));
//! // 同時刻では CC が note on より前に来る（鳴り始めに modulation 値が乗るように）。
//! assert_eq!(events[0].message, [0xB0, MODULATION_CC, lfo.value_at(1.5)]);
//! assert_eq!(events[1].message, [0x90, 60, 100]);
//! ```

mod control_change;
mod lfo;
mod repeat;
mod velocity;

#[cfg(test)]
mod tests;

pub use control_change::{insert_control_change, MODULATION_CC};
pub use lfo::TriangleLfo;
pub use repeat::shift;
pub use velocity::override_note_velocity;

/// フレーズ先頭を 0 秒とした MIDI イベント。
///
/// `message` は生の 3 バイト。SHM の wire format がイベント種別を持たず生 3 バイトを運ぶので、
/// ここでも種別を型で分けない（CC も pitch bend もそのまま載る）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedMidiEvent {
    pub seconds: f64,
    pub message: [u8; 3],
}

/// 変換を掛ける絶対秒の範囲 `[start, end)`。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub start_seconds: f64,
    pub end_seconds: f64,
}

impl Span {
    pub fn new(start_seconds: f64, end_seconds: f64) -> Self {
        Self {
            start_seconds,
            end_seconds,
        }
    }

    /// 長さ。`end <= start` なら 0。
    pub fn duration_seconds(&self) -> f64 {
        (self.end_seconds - self.start_seconds).max(0.0)
    }
}

/// note off か。`0x80` 系に加え、**velocity 0 の note on** も note off として扱う。
pub(crate) fn is_note_off(message: &[u8; 3]) -> bool {
    matches!(message[0] & 0xF0, 0x80) || (message[0] & 0xF0 == 0x90 && message[2] == 0)
}

/// 実際に音を出す note on か（velocity 0 は note off なので除く）。
pub(crate) fn is_note_on(message: &[u8; 3]) -> bool {
    message[0] & 0xF0 == 0x90 && message[2] != 0
}

/// 同時刻での並び順。小さいほど先。
fn playback_rank(message: &[u8; 3]) -> u8 {
    if is_note_off(message) {
        0
    } else if is_note_on(message) {
        2
    } else {
        1
    }
}

/// 同時刻の並び順を決める。**note off → CC → note on** の順。
///
/// CC が note on より後だと、鳴り始めの 1 音に modulation 値が乗らない。note off を先頭に
/// 置くのは、同じ音高の連打で新しい note on を消さないため。
///
/// stable sort なので、同時刻・同種のイベントは呼び出し側が積んだ順を保つ。
pub fn sort_for_playback(events: &mut [TimedMidiEvent]) {
    events.sort_by(|a, b| {
        a.seconds
            .total_cmp(&b.seconds)
            .then_with(|| playback_rank(&a.message).cmp(&playback_rank(&b.message)))
    });
}
