//! selector の中の auto reverb（`e` のルール overlay と `E` の on/off）と、ルールが変わったときの
//! 鳴らし直し。操作と描画は `cmrt_patch_select` の [`cmrt_patch_select::AutoReverbPanel`] をそのまま使う。

use cmrt_patch_select::{auto_reverb::AutoReverbRules, AutoReverbKey};
use crossterm::event::KeyEvent;

use crate::{GridSequencerAction, GridSequencerScreen};

impl GridSequencerScreen {
    /// auto reverb のキーなら処理して action を返す。ルール overlay が開いている間は全キーを食う。
    pub(super) fn handle_patch_selector_auto_reverb_key(
        &mut self,
        key: KeyEvent,
    ) -> Option<GridSequencerAction> {
        let selector = self.patch_selector.as_mut()?;
        match selector.auto_reverb.handle_key(key)? {
            AutoReverbKey::Handled => Some(GridSequencerAction::Continue),
            AutoReverbKey::RulesChanged => {
                let rules = selector.auto_reverb.rules().clone();
                self.apply_auto_reverb_rules(rules.clone());
                Some(GridSequencerAction::SaveAutoReverb(rules))
            }
        }
    }

    /// ルールを差し替え、chain が変わる行だけ準備し直す。selector の行は試聴中の音色で比べる。
    fn apply_auto_reverb_rules(&mut self, rules: AutoReverbRules) {
        let loaded = (0..self.state.instance_count())
            .map(|instance| self.loaded_patch(instance))
            .collect::<Vec<_>>();
        let before = loaded
            .iter()
            .map(|patch| self.auto_reverb.patch(patch.as_deref()))
            .collect::<Vec<_>>();
        self.auto_reverb.set_rules(rules);
        for (instance, (patch, before)) in loaded.iter().zip(before).enumerate() {
            if self.auto_reverb.patch(patch.as_deref()) != before {
                self.prepare_patch(instance, patch.as_deref(), "auto-reverb");
            }
        }
    }

    /// 行に今載っている音色。selector で試聴中の行は試聴中の音色。
    fn loaded_patch(&self, instance: usize) -> Option<String> {
        match &self.patch_selector {
            Some(selector) if selector.instance == instance => selector.previewed_patch.clone(),
            _ => self
                .state
                .instances()
                .get(instance)
                .and_then(|item| item.patch.clone()),
        }
    }
}
