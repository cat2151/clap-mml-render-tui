//! 保存値・undo snapshot から instance/lane を復元する。

use super::{measure_lane, velocity, GridInstance, GridState};

impl GridState {
    pub(crate) fn restore_instances(&mut self, mut instances: Vec<GridInstance>) -> bool {
        if instances.len() != self.instances.len() {
            return false;
        }
        for instance in &mut instances {
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
