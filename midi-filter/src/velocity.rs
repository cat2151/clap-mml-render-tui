//! note on の velocity を LFO で乗っ取る。
//!
//! MML overlay は velocity を自分で決めていない。`mmlabc-to-smf` が書いた SMF の
//! note on の 3 バイト目（既定 127、`v8` 等で変わる）をそのまま運んでいるだけなので、
//! wire へ出る直前のここで上書きするのが正しい差し込み点。

use crate::{is_note_on, TimedMidiEvent, TriangleLfo};

#[cfg(test)]
mod tests;

/// velocity 0 は note off になってしまうので、下限は 1。
const MIN_VELOCITY: u8 = 1;
const MAX_VELOCITY: u8 = 127;

/// note on の velocity を、**その音自身の時刻**の LFO 値で置き換える。
///
/// note off（`0x80` 系）と velocity 0 の note on には触らない。触ると note が切れなくなる。
/// LFO の下限が 0 でも 1 へ丸めるので、無音の note on は作らない。
pub fn override_note_velocity(events: &mut [TimedMidiEvent], lfo: &TriangleLfo) {
    for event in events.iter_mut() {
        if !is_note_on(&event.message) {
            continue;
        }
        event.message[2] = lfo
            .value_at(event.seconds)
            .clamp(MIN_VELOCITY, MAX_VELOCITY);
    }
}
