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

    /// track 数を変えて再起動する準備。既存の行はそのまま引き継ぐ。
    ///
    /// 増やしたぶんの行は抽選して埋める。空のまま足すと、HOLD では譜面を引き直さないので
    /// 増やした行が延々と無音のままになる（`r` を押すまで気づけない）。
    pub(crate) fn resize_for_restart(
        &mut self,
        instance_count: usize,
        patches: &[(String, String)],
    ) {
        let mut instances = self.state.instances().to_vec();
        let previous_count = instances.len();
        while instances.len() < instance_count {
            instances.push(GridInstance::new(instances.len()));
        }
        instances.truncate(instance_count);
        if let Some(added) = instances.get_mut(previous_count..) {
            crate::randomize_instance_slice(added, patches);
        }

        self.finish();
        let mut state = GridState::with_instance_count(instance_count);
        let restored = state.restore_instances(instances);
        debug_assert!(restored);
        self.state = state;
        self.pending_undo = None;
        self.undo = None;
        // instance 番号の指す先が変わるので、音型と patch のカーソルは引き継がない。
        self.reset_arp_patterns();
        self.reset_bass_pattern();
        self.reset_patch_bags();
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
