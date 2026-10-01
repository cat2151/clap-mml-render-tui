//! 行全体のパラメータ（連続値の CC）。値は [`RuleTable`] が持ち、sfz の既定と違う値だけを
//! 演奏のいちばん早い note on で送り、演奏の最後の note off で既定へ戻す。

use std::collections::BTreeMap;

use crate::{Note, RuleTable, TimedMidiEvent};

/// パラメータ 1 つ。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Param {
    pub cc: u8,
    /// イベント一覧とパラメータ overlay に出す名前。
    pub name: &'static str,
    /// sfz の `set_ccN`。この値のときは送らない。
    pub default: u8,
}

/// パラメータ overlay（`u`）に並べる順。
pub const PARAMS: [Param; 9] = [
    param(21, "vibrato speed", 95),
    param(22, "mute length", 51),
    param(28, "unison speed", 44),
    param(29, "resonance", 44),
    param(46, "tension", 0),
    param(48, "magnet", 64),
    param(52, "bend start", 0),
    param(53, "bend speed", 0),
    param(112, "trill speed", 0),
];

/// パラメータ overlay の h/l 1 回で変える幅。
pub const PARAM_STEP: u8 = 8;

const PARAM_MAX: u8 = 127;

const fn param(cc: u8, name: &'static str, default: u8) -> Param {
    Param { cc, name, default }
}

/// `cc` のパラメータ。[`PARAMS`] に無ければ `None`。
pub fn param_of(cc: u8) -> Option<Param> {
    PARAMS.iter().copied().find(|param| param.cc == cc)
}

/// JSON から読んだ値のうち、[`PARAMS`] に在り、既定と違い、0〜127 のものだけ残す。
pub(crate) fn sanitized(params: BTreeMap<u8, u8>) -> BTreeMap<u8, u8> {
    params
        .into_iter()
        .filter(|(cc, value)| {
            *value <= PARAM_MAX && param_of(*cc).is_some_and(|param| param.default != *value)
        })
        .collect()
}

impl RuleTable {
    /// `cc` の値。設定していなければ sfz の既定。[`PARAMS`] に無い `cc` は 0。
    pub fn param(&self, cc: u8) -> u8 {
        self.params
            .get(&cc)
            .copied()
            .or_else(|| param_of(cc).map(|param| param.default))
            .unwrap_or(0)
    }

    /// `cc` の値を `delta` だけ変える（0〜127 に収める）。値が変わったら `true`。
    /// 既定と同じ値になったら持たない（JSON にも出ない）。[`PARAMS`] に無い `cc` は変えない。
    pub fn step_param(&mut self, cc: u8, delta: i16) -> bool {
        let Some(param) = param_of(cc) else {
            return false;
        };
        let before = self.param(cc);
        let after = (i16::from(before) + delta).clamp(0, i16::from(PARAM_MAX)) as u8;
        if after == param.default {
            self.params.remove(&cc);
        } else {
            self.params.insert(cc, after);
        }
        after != before
    }

    /// 既定と違う値の (CC, 値)。CC の小さい順。
    pub fn changed_params(&self) -> impl Iterator<Item = (u8, u8)> + '_ {
        self.params.iter().map(|(cc, value)| (*cc, *value))
    }
}

/// 既定と違うパラメータを、`notes` のいちばん早い note on で送り、最後の note off で既定へ戻す。
/// 音が無ければ空。channel は先頭の音のもの。
pub(crate) fn param_events(notes: &[Note], rules: &RuleTable) -> Vec<TimedMidiEvent> {
    let Some(first) = notes.first() else {
        return Vec::new();
    };
    let start = notes.iter().map(|n| n.on_seconds).fold(f64::MAX, f64::min);
    let end = notes.iter().map(|n| n.off_seconds).fold(0.0, f64::max);
    let status = 0xB0 | first.channel;
    let mut out = Vec::new();
    for (cc, value) in rules.changed_params() {
        let default = param_of(cc).map_or(0, |param| param.default);
        out.push(TimedMidiEvent {
            seconds: start,
            message: [status, cc, value],
        });
        out.push(TimedMidiEvent {
            seconds: end,
            message: [status, cc, default],
        });
    }
    out
}

#[cfg(test)]
mod tests;
