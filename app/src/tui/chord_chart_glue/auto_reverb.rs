//! Chord Chart の音色準備に載せる auto reverb の effect chain。
//!
//! Chord Chart の音色（Chord role・Bass role）には effect を保存する欄が無い。selector の試聴も、
//! section の演奏も、degrees 編集 overlay の演奏も、[`TuiApp::chord_chart_live_patch`] で
//! その時点のルールを当てた chain を載せる。

use cmrt_mml_overlay::LivePatch;
use cmrt_patch_select::{
    auto_reverb::{resolve_in_catalog, AutoReverbRules},
    AutoReverbHost,
};

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

    /// selector のルール overlay へ渡すもの。Chord Chart の音色には chain の欄が無いので
    /// `existing_chain` は無い。
    pub(in crate::tui) fn chord_chart_auto_reverb_host(&self) -> AutoReverbHost {
        AutoReverbHost {
            rules: self.chord_chart_auto_reverb_rules.clone(),
            effect_plugins: self.effect_plugins.clone(),
            existing_chain: false,
        }
    }

    /// 音色 1 つに掛ける chain。音色・patch catalog が無ければ空（勝手に足さない）。
    pub(in crate::tui) fn chord_chart_effect_chain(&self, patch: Option<&str>) -> String {
        let Some(patch) = patch else {
            return String::new();
        };
        match &*self.patch_load_state.lock().unwrap() {
            PatchLoadState::Ready(snapshot) => resolve_in_catalog(
                patch,
                snapshot,
                self.effect_plugins.catalog(),
                &self.chord_chart_auto_reverb_rules,
            )
            .effect_chain(),
            PatchLoadState::Loading | PatchLoadState::Err(_) => String::new(),
        }
    }

    /// server へ準備させる音色と chain。
    pub(in crate::tui) fn chord_chart_live_patch(&self, patch: Option<&str>) -> LivePatch {
        LivePatch::with_effect_chain(patch, &self.chord_chart_effect_chain(patch))
    }
}
