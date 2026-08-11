//! 保存値・undo snapshot から instance/lane を復元する。

use super::{measure_lane, velocity, GridInstance, GridLaneMode, GridState};

impl GridState {
    pub(crate) fn restore_instances(&mut self, mut instances: Vec<GridInstance>) -> bool {
        if instances.len() != self.instances.len() {
            return false;
        }
        for (index, instance) in instances.iter_mut().enumerate() {
            restore_lane_mode(index, instance);
            instance.normalize();
        }
        let stored_lane_count = instances
            .iter()
            .map(|instance| instance.lanes.len())
            .sum::<usize>();
        self.instances = instances;
        self.velocity = measure_lane::MeasureLane::new(
            stored_lane_count,
            velocity::VELOCITY_CHOICES,
            measure_lane::LaneCoverage::SoundingCells,
        );
        self.refresh_lane_display_patterns();
        self.discard_pending_cycle();
        true
    }

    pub(crate) fn clear_unavailable_patches(&mut self, patches: &[(String, String)]) {
        for instance in &mut self.instances {
            let available = instance
                .patch
                .as_ref()
                .is_none_or(|saved| patches.iter().any(|(patch, _)| patch == saved));
            if !available {
                instance.patch = None;
            }
        }
    }
}

/// `lane_mode` は行番号から決まるもので、ユーザーが編集できる値ではない。保存値を
/// 信じずに毎回 index の既定へ寄せ直す。
///
/// bass 行（[`super::BASS_ROW`]）が 4 voice の既定行だった頃のセッションを引き継ぐために要る。
/// そのままだと bass 行に余った lane がぶら下がり、4 voice の行が1つも無くなって
/// アルペジオ（[`super::arpeggio`]）が書けなくなる。逆に bass 行が 1 lane だった頃の
/// セッションは、`normalize()` の resize で octave 上の lane が空のまま足される。
fn restore_lane_mode(index: usize, instance: &mut GridInstance) {
    let lane_mode = GridInstance::new(index).lane_mode;
    if instance.lane_mode == lane_mode {
        return;
    }
    instance.lane_mode = lane_mode;
    if lane_mode != GridLaneMode::ChordVoices4 {
        // 転回は 4 voice の行だけが持つ状態。他の mode へ落とすときは一緒に捨てる。
        instance.voicing_rotation = 0;
    }
}

#[cfg(test)]
mod tests;
