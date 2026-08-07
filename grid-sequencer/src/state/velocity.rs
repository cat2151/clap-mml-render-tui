//! note velocity レーン。小節頭に抽選した値を note on に載せる。
//!
//! 抽選と表示の仕組みそのものは [`super::measure_lane`] にある。

use super::{
    measure_lane::{pattern::MeasurePattern, LaneDisplayRow},
    GridState,
};

/// このレーンが取りうる `[低い値, 高い値]`。
pub(super) const VELOCITY_CHOICES: [u8; 2] = [100, 127];

impl GridState {
    /// 実発音中の小節に対応する velocity grid。`None` のセルは発音しないステップ。
    #[cfg(test)]
    pub(crate) fn velocity_display(&self) -> &[LaneDisplayRow] {
        self.velocity.display()
    }

    /// 現在可視なNOTE rowと同じ順序へ、stored laneの表示値を写す。
    pub(crate) fn visible_velocity_display(&self) -> Vec<LaneDisplayRow> {
        self.visible_note_rows()
            .into_iter()
            .filter_map(|row| {
                self.stored_lane_index(row.address)
                    .and_then(|index| self.velocity.display().get(index).copied())
            })
            .collect()
    }

    /// 実発音中の小節のパターン名。再生前は `None`。
    pub(crate) fn velocity_pattern_label(&self) -> Option<&'static str> {
        Some(match self.velocity.pattern()? {
            MeasurePattern::Random => "random",
            MeasurePattern::Ramp { ascending: true } => "vel up",
            MeasurePattern::Ramp { ascending: false } => "vel down",
        })
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
