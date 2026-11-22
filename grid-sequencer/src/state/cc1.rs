//! MIDI CC1（modulation）レーン。通常は小節頭に抽選し、音高が変わる音色は 0 に固定する。
//!
//! modulation は note on の瞬間だけでなく発音中にも効くので、note on するセルに
//! 限らず毎ステップ送る。伸ばしている音や chord mode の全音符にもランプがかかる。
//!
//! 抽選と表示の仕組みそのものは [`super::measure_lane`] にある。

use std::sync::Arc;

use cmrt_tui_core::patch_load::PatchLoadState;

use super::{
    measure_lane::{pattern::MeasurePattern, LaneDisplayRow},
    GridState,
};

const CONTROL_CHANGE: u8 = 0xB0;
const MODULATION_CC: u8 = 1;
/// このレーンが取りうる `[低い値, 高い値]`。
pub(super) const CC1_CHOICES: [u8; 2] = [0, 127];

impl GridState {
    pub(crate) fn observe_cc1_catalog(&mut self, patch_load: &PatchLoadState) {
        if let PatchLoadState::Ready(snapshot) = patch_load {
            self.cc1_pitch_change_patches = Arc::clone(snapshot.cc1_pitch_change_patches());
        }
    }

    pub(crate) fn cc1_is_fixed(&self, patch: Option<&str>) -> bool {
        patch.is_some_and(|patch| self.cc1_pitch_change_patches.contains(patch))
    }

    pub(crate) fn set_cc1_preview_patch(&mut self, preview: Option<(u8, String)>) {
        self.cc1_preview_patch = preview;
    }

    fn instance_cc1_is_fixed(&self, instance_id: u8, patch: Option<&str>) -> bool {
        let patch = match &self.cc1_preview_patch {
            Some((preview_id, preview)) if *preview_id == instance_id => Some(preview.as_str()),
            _ => patch,
        };
        self.cc1_is_fixed(patch)
    }

    /// 実発音中の小節に対応する CC1 grid。全stepで送るので、再生中は全セルが埋まる。
    pub(crate) fn cc1_display(&self) -> Vec<LaneDisplayRow> {
        self.cc1
            .display()
            .iter()
            .zip(self.display_instances())
            .enumerate()
            .map(|(instance, (values, item))| {
                if self.instance_cc1_is_fixed(
                    self.display_instance_id(instance),
                    item.patch.as_deref(),
                ) {
                    values.map(|value| value.map(|_| 0))
                } else {
                    *values
                }
            })
            .collect()
    }

    /// 組み立て中の列で全行へ送る CC1。note on の有無は見ない。
    pub(super) fn cc1_messages_for_step(&self) -> Vec<(u8, [u8; 3])> {
        let step = self.schedule_index;
        self.patches()
            .enumerate()
            .map(|(instance, (id, patch))| {
                let value = if self.instance_cc1_is_fixed(id, patch) {
                    0
                } else {
                    self.cc1.value_at(instance, step)
                };
                (id, control_change(value))
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
