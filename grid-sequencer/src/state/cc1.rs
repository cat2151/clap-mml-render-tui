//! MIDI CC1（modulation）レーン。小節頭に抽選した値を note on の直前へ送る。
//!
//! 抽選と表示の仕組みそのものは [`super::measure_lane`] にある。

use super::{measure_lane::LaneDisplayRow, GridState};

const CONTROL_CHANGE: u8 = 0xB0;
const MODULATION_CC: u8 = 1;
/// 小節ごとに、セル単位で選び直す2値。
pub(super) const CC1_CHOICES: [u8; 2] = [0, 127];

impl GridState {
    /// 実発音中の小節に対応する CC1 grid。`None` のセルでは CC1 を送らない。
    pub(crate) fn cc1_display(&self) -> &[LaneDisplayRow] {
        self.cc1.display()
    }
}

pub(super) fn control_change(value: u8) -> [u8; 3] {
    [CONTROL_CHANGE, MODULATION_CC, value]
}

#[cfg(test)]
mod tests;
