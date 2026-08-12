//! Grid Sequencer の1段 undo。

use crate::{CycleRandom, GridInstance, GridSequencerContext, GridSequencerScreen};

#[derive(Clone, Debug)]
pub(crate) struct UndoSnapshot {
    instances: Vec<GridInstance>,
    cycle_random: CycleRandom,
}

impl GridSequencerScreen {
    pub(crate) fn capture_undo(&self) -> UndoSnapshot {
        UndoSnapshot {
            instances: self.state.instances().to_vec(),
            cycle_random: self.cycle_random,
        }
    }

    pub(crate) fn begin_undo_gesture(&mut self) {
        self.pending_undo = Some(self.capture_undo());
    }

    pub(crate) fn finish_undo_gesture(&mut self, changed: bool) {
        let snapshot = self.pending_undo.take();
        if changed {
            if let Some(snapshot) = snapshot {
                self.commit_undo(snapshot);
            }
        }
    }

    pub(crate) fn commit_undo(&mut self, snapshot: UndoSnapshot) {
        if snapshot.instances != self.state.instances()
            || snapshot.cycle_random != self.cycle_random
        {
            self.undo = Some(snapshot);
        }
    }

    pub(crate) fn undo(&mut self, ctx: &GridSequencerContext<'_>) {
        self.cancel_mouse_gesture();
        let Some(snapshot) = self.undo.take() else {
            return;
        };
        let patches_before = self
            .state
            .instances()
            .iter()
            .map(|instance| instance.patch.clone())
            .collect::<Vec<_>>();
        let restored = self.state.restore_instances(snapshot.instances);
        debug_assert!(restored);
        self.restore_cycle_random(snapshot.cycle_random);
        self.cancel_cycle_swap_preserving_drain();

        let patch_instances_changed = self
            .state
            .instances()
            .iter()
            .enumerate()
            .filter_map(|(instance, item)| {
                (item.patch != patches_before[instance]).then_some(instance)
            })
            .collect::<Vec<_>>();
        match patch_instances_changed.as_slice() {
            [instance] => self.prepare_instance_patch(*instance),
            [] => {}
            _ => self.prepare_connection_or_start_server(ctx),
        }
    }
}

#[cfg(test)]
mod tests;
