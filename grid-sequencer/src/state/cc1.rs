//! MIDI CC1（modulation）レーン。小節頭に抽選した値を、全stepで全行へ送る。
//!
//! modulation は note on の瞬間だけでなく発音中にも効くので、note on するセルに
//! 限らず毎ステップ送る。伸ばしている音や chord mode の全音符にもランプがかかる。
//!
//! 抽選と表示の仕組みそのものは [`super::measure_lane`] にある。

use super::{
    measure_lane::{pattern::MeasurePattern, LaneDisplayRow},
    GridState,
};

const CONTROL_CHANGE: u8 = 0xB0;
const MODULATION_CC: u8 = 1;
/// このレーンが取りうる `[低い値, 高い値]`。
pub(super) const CC1_CHOICES: [u8; 2] = [0, 127];

impl GridState {
    /// 実発音中の小節に対応する CC1 grid。全stepで送るので、再生中は全セルが埋まる。
    pub(crate) fn cc1_display(&self) -> &[LaneDisplayRow] {
        self.cc1.display()
    }

    /// 組み立て中の列で全行へ送る CC1。note on の有無は見ない。
    pub(super) fn cc1_messages_for_step(&self) -> Vec<(u8, [u8; 3])> {
        let step = self.schedule_index;
        (0..self.instance_count())
            .map(|instance| {
                (
                    self.instance_id(instance),
                    control_change(self.cc1.value_at(instance, step)),
                )
            })
            .collect()
    }

    /// 実発音中の小節のパターン名。再生前は `None`。
    pub(crate) fn cc1_pattern_label(&self) -> Option<&'static str> {
        Some(match self.cc1.pattern()? {
            MeasurePattern::Random => "random",
            MeasurePattern::Ramp { ascending: true } => "cc1 up",
            MeasurePattern::Ramp { ascending: false } => "cc1 down",
        })
    }
}

pub(super) fn control_change(value: u8) -> [u8; 3] {
    [CONTROL_CHANGE, MODULATION_CC, value]
}

#[cfg(test)]
mod tests;
