//! Chord Chart の音色準備に載せる effect chain。
//!
//! Chord role の音色には、画面が持つ chain を 1 本掛け、auto reverb と合成する（合成規則は
//! [`HostChain`]）。Bass role の音色には auto reverb だけを掛ける。selector の試聴も、
//! section の演奏も、degrees 編集 overlay の演奏も、[`TuiApp::chord_chart_live_patch`] で
//! その時点のルールを当てた chain を載せる。

use cmrt_core::EFFECT_CHAIN_JSON_KEY;
use cmrt_mml_overlay::LivePatch;
use cmrt_patch_select::{
    auto_reverb::{resolve_in_catalog, AutoReverb, AutoReverbRules, HostChain},
    AutoReverbHost,
};
use cmrt_patches::PatchRole;
use serde_json::Value;

use crate::tui::{PatchLoadState, TuiApp};

impl TuiApp<'_> {
    /// 保存済みのルールを読み直す。他の画面の selector で変えたルールも、これで効く。
    pub(in crate::tui) fn reload_chord_chart_auto_reverb_rules(&mut self) {
        self.chord_chart_auto_reverb_rules =
            crate::tui::auto_reverb_settings::load_auto_reverb_rules(&self.effect_plugins);
    }

    /// selector で変えたルールを保存し、以後の試聴と演奏へ効かせる。
    pub(in crate::tui) fn set_chord_chart_auto_reverb_rules(&mut self, rules: AutoReverbRules) {
        crate::tui::auto_reverb_settings::save_auto_reverb_rules(&rules);
        self.chord_chart_auto_reverb_rules = rules;
    }

    /// selector のルール overlay へ渡すもの。chain は `role` の音色に掛ける手動 chain。
    pub(in crate::tui) fn chord_chart_auto_reverb_host(&self, role: PatchRole) -> AutoReverbHost {
        AutoReverbHost {
            rules: self.chord_chart_auto_reverb_rules.clone(),
            effect_plugins: self.effect_plugins.clone(),
            chain: self.chord_chart_host_chain(role),
        }
    }

    /// `role` の音色に掛ける手動 chain。Chord role だけが持つ。
    fn chord_chart_host_chain(&self, role: PatchRole) -> HostChain {
        if role != PatchRole::Chord {
            return HostChain::default();
        }
        self.host_chain_of(&self.chord_chart_effect_chain_stages)
    }

    fn host_chain_of(&self, stages: &[Value]) -> HostChain {
        if stages.is_empty() {
            return HostChain::default();
        }
        let json = serde_json::json!({ EFFECT_CHAIN_JSON_KEY: stages });
        HostChain::from_json(Some(&json), self.effect_plugins.catalog())
    }

    /// `role` の音色 1 つに掛ける chain。音色が無ければ空。patch catalog が無ければ
    /// auto reverb は足さない（勝手に足さない）。
    pub(in crate::tui) fn chord_chart_effect_chain(
        &self,
        role: PatchRole,
        patch: Option<&str>,
    ) -> String {
        self.effect_chain_with_auto_reverb(self.chord_chart_host_chain(role), patch)
    }

    /// 手動 chain `host` に、`patch` の auto reverb を合成した chain。
    fn effect_chain_with_auto_reverb(&self, host: HostChain, patch: Option<&str>) -> String {
        let Some(patch) = patch else {
            return String::new();
        };
        let auto_reverb = match &*self.patch_load_state.lock().unwrap() {
            PatchLoadState::Ready(snapshot) => resolve_in_catalog(
                patch,
                snapshot,
                self.effect_plugins.catalog(),
                &self.chord_chart_auto_reverb_rules,
            ),
            PatchLoadState::Loading | PatchLoadState::Err(_) => AutoReverb::NoCatalog,
        };
        let stage = match &auto_reverb {
            AutoReverb::Apply { stage, .. } => Some(stage),
            AutoReverb::Dry { .. }
            | AutoReverb::Builtin
            | AutoReverb::Off
            | AutoReverb::NoCatalog => None,
        };
        let chain = host.with_auto_reverb(stage);
        if chain.is_empty() {
            String::new()
        } else {
            Value::Array(chain).to_string()
        }
    }

    /// server へ準備させる `role` の音色と chain。
    pub(in crate::tui) fn chord_chart_live_patch(
        &self,
        role: PatchRole,
        patch: Option<&str>,
    ) -> LivePatch {
        LivePatch::with_effect_chain(patch, &self.chord_chart_effect_chain(role, patch))
    }

    /// Chord 音色に、確定前の手動 chain `stages` を掛けて準備させるもの。`x` の試聴用。
    pub(in crate::tui) fn chord_chart_live_patch_with_stages(&self, stages: &[Value]) -> LivePatch {
        let patch = self.chord_chart_patch.as_deref();
        let chain = self.effect_chain_with_auto_reverb(self.host_chain_of(stages), patch);
        LivePatch::with_effect_chain(patch, &chain)
    }
}
