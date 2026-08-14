//! instance / lane への参照と、画面に出す行への写像。
//!
//! 「格納されている lane」と「画面に見えている行」は同じではない。chord mode では
//! 和音の行が summary 1 行に畳まれ、多声の行は lane ごとに展開される。その写像を
//! ここへ集めてあり、描画も mouse の当たり判定も必ずここを通る。

use super::{
    GridInstance, GridLane, GridState, LaneAddress, VisibleNoteRow, VisibleRowKind, CHORD_ROW,
};

impl GridState {
    pub fn stored_lane_count(&self) -> usize {
        self.instances
            .iter()
            .map(|instance| instance.lanes.len())
            .sum()
    }

    pub fn visible_lane_count(&self) -> usize {
        self.visible_note_rows().len()
    }

    pub fn instances(&self) -> &[GridInstance] {
        &self.instances
    }

    pub fn instances_mut(&mut self) -> &mut [GridInstance] {
        &mut self.instances
    }

    pub fn lane(&self, address: LaneAddress) -> Option<&GridLane> {
        self.instances
            .get(address.instance)?
            .lanes
            .get(address.lane)
    }

    pub fn lane_mut(&mut self, address: LaneAddress) -> Option<&mut GridLane> {
        self.instances
            .get_mut(address.instance)?
            .lanes
            .get_mut(address.lane)
    }

    pub fn visible_note_rows(&self) -> Vec<VisibleNoteRow> {
        Self::visible_note_rows_from(&self.instances, self.chord.is_some())
    }

    pub(super) fn visible_note_rows_from(
        instances: &[GridInstance],
        chord_on: bool,
    ) -> Vec<VisibleNoteRow> {
        instances
            .iter()
            .enumerate()
            .flat_map(|(instance_index, instance)| {
                let count = if chord_on && instance_index == CHORD_ROW {
                    // chord専用instanceは保存modeによらず、常にsummary 1 rowだけを出す。
                    1.min(instance.lanes.len())
                } else if chord_on {
                    instance.lanes.len()
                } else {
                    1.min(instance.lanes.len())
                };
                let lanes = if chord_on && instance.lane_mode.stacks_high_notes_on_top() {
                    // piano rollと同じく高音を上、lane 0（既定root）を最下段へ置く。
                    (0..count).rev().collect::<Vec<_>>()
                } else {
                    (0..count).collect::<Vec<_>>()
                };
                lanes.into_iter().map(move |lane| VisibleNoteRow {
                    address: LaneAddress::new(instance_index, lane),
                    kind: if chord_on && instance_index == CHORD_ROW {
                        VisibleRowKind::ChordSummary
                    } else {
                        VisibleRowKind::Normal
                    },
                })
            })
            .collect()
    }

    pub fn stored_lane_addresses(&self) -> Vec<LaneAddress> {
        self.instances
            .iter()
            .enumerate()
            .flat_map(|(instance, item)| {
                (0..item.lanes.len()).map(move |lane| LaneAddress::new(instance, lane))
            })
            .collect()
    }

    pub(super) fn stored_lane_index(&self, address: LaneAddress) -> Option<usize> {
        let instance = self.instances.get(address.instance)?;
        (address.lane < instance.lanes.len()).then(|| {
            self.instances[..address.instance]
                .iter()
                .map(|item| item.lanes.len())
                .sum::<usize>()
                + address.lane
        })
    }
}
