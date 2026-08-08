//! 編集可能な grid の復元・保存と instance 数変更時の引継ぎ。

use crate::{GridInstance, GridSequencerScreen, GridState, PatternEvolution};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridSequencerSession {
    pub instances: Vec<GridInstance>,
    pub pattern_evolution: PatternEvolution,
}

impl GridSequencerSession {
    pub fn new(instances: Vec<GridInstance>, pattern_evolution: PatternEvolution) -> Self {
        Self {
            instances,
            pattern_evolution,
        }
    }
}

impl GridSequencerScreen {
    pub fn session_state(&self) -> Option<GridSequencerSession> {
        self.grid_ready.then(|| GridSequencerSession {
            instances: self.state.instances().to_vec(),
            pattern_evolution: self.pattern_evolution,
        })
    }

    pub(crate) fn resize_for_restart(&mut self, instance_count: usize) {
        let mut instances = self.state.instances().to_vec();
        while instances.len() < instance_count {
            instances.push(GridInstance::new(instances.len()));
        }
        instances.truncate(instance_count);

        self.finish();
        let mut state = GridState::with_instance_count(instance_count);
        let restored = state.restore_instances(instances);
        debug_assert!(restored);
        self.state = state;
        self.pending_undo = None;
        self.undo = None;
        // instance 番号の指す先が変わるので、音型のカーソルは引き継がない。
        self.reset_arp_patterns();
    }

    pub(crate) fn validate_restored_patches(&mut self, patches: &[(String, String)]) -> bool {
        if !self.restored_patches_pending {
            return false;
        }
        self.restored_patches_pending = false;
        self.state.clear_unavailable_patches(patches);
        true
    }
}

#[cfg(test)]
mod tests;
