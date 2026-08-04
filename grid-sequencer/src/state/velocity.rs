//! note velocity レーン。小節頭に抽選した値を note on に載せる。
//!
//! 抽選と表示の仕組みそのものは [`super::measure_lane`] にある。

use super::{measure_lane::LaneDisplayRow, GridState};

/// 小節ごとに、セル単位で選び直す2値。
pub(super) const VELOCITY_CHOICES: [u8; 2] = [100, 127];

impl GridState {
    /// 実発音中の小節に対応する velocity grid。`None` のセルは発音しないステップ。
    pub(crate) fn velocity_display(&self) -> &[LaneDisplayRow] {
        self.velocity.display()
    }
}

/// 譜面を比較するテスト向けに、抽選された velocity を既定値へ均す。
///
/// 実際に抽選値が載っているかは [`tests`] で確かめるので、他のテストは
/// note number と送信順だけを見られるようにする。
#[cfg(test)]
pub(super) fn normalize_velocity(mut message: [u8; 3]) -> [u8; 3] {
    if message[0] == super::NOTE_ON {
        message[2] = VELOCITY_CHOICES[0];
    }
    message
}

#[cfg(test)]
mod tests;
