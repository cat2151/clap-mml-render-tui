//! grid の音色準備に載せる auto reverb の effect chain。
//!
//! grid には effect を保存する欄が無い。音色を server へ準備させる口（全 instance・1 行・
//! 待機 bank）は、試聴も演奏も [`GridAutoReverb::patch`] で、その時点のルールを当てた chain を載せる。

use std::sync::Arc;

use cmrt_core::EffectPlugins;
use cmrt_patch_select::{
    auto_reverb::{resolve, resolve_in_catalog, AutoReverb, AutoReverbRules},
    AutoReverbHost, AutoReverbStatus,
};
use cmrt_patches::PatchRoleIndex;
use cmrt_tui_core::patch_load::{PatchCatalogSnapshot, PatchLoadState};

use crate::GridPatch;

/// ルールと、解決に要る effect catalog・patch catalog。
#[derive(Default)]
pub(crate) struct GridAutoReverb {
    rules: AutoReverbRules,
    effect_plugins: EffectPlugins,
    /// 直近の ctx の patch catalog。Ready でなければ `None`。
    catalog: Option<Arc<PatchCatalogSnapshot>>,
    /// ルール overlay の effect list で試聴中の `(音色, effect chain)`。その音色にはルールに
    /// 代えてこの chain を載せる（空は dry）。
    audition: Option<(String, String)>,
}

impl GridAutoReverb {
    pub(crate) fn new(effect_plugins: EffectPlugins) -> Self {
        Self {
            effect_plugins,
            ..Self::default()
        }
    }

    /// ctx の patch catalog を覚える。ctx を受け取らない送信経路もこれで解決する。
    pub(crate) fn observe(&mut self, patch_load: &PatchLoadState) {
        self.catalog = match patch_load {
            PatchLoadState::Ready(snapshot) => Some(Arc::clone(snapshot)),
            PatchLoadState::Loading | PatchLoadState::Err(_) => None,
        };
    }

    pub(crate) fn set_rules(&mut self, rules: AutoReverbRules) {
        self.rules = rules;
    }

    pub(crate) fn set_audition(&mut self, audition: Option<(String, String)>) {
        self.audition = audition;
    }

    /// selector のルール overlay へ渡すもの。grid の行には chain の欄が無いので、chain は空。
    pub(crate) fn host(&self) -> AutoReverbHost {
        AutoReverbHost {
            rules: self.rules.clone(),
            effect_plugins: self.effect_plugins.clone(),
            chain: Default::default(),
        }
    }

    /// 音色 1 つに掛ける auto reverb。patch catalog が無ければ内蔵扱い（勝手に足さない）。
    pub(crate) fn resolve(&self, patch: &str) -> AutoReverb {
        let catalog = self.effect_plugins.catalog();
        match &self.catalog {
            Some(snapshot) => resolve_in_catalog(patch, snapshot, catalog, &self.rules),
            None => resolve(
                patch,
                true,
                &PatchRoleIndex::default(),
                catalog,
                &self.rules,
            ),
        }
    }

    /// server へ準備させる音色と chain。音色が無ければ chain も無い。
    pub(crate) fn patch(&self, patch: Option<&str>) -> GridPatch {
        GridPatch {
            patch: patch.map(str::to_string),
            effect_chain: patch
                .map(|patch| match &self.audition {
                    Some((auditioned, chain)) if auditioned == patch => chain.clone(),
                    _ => self.resolve(patch).effect_chain(),
                })
                .unwrap_or_default(),
        }
    }

    /// selector の表示行に出す状態。
    pub(crate) fn status(&self, patch: Option<&str>) -> AutoReverbStatus {
        match patch {
            _ if !self.rules.enabled() => AutoReverbStatus::Resolved(AutoReverb::Off),
            Some(patch) => AutoReverbStatus::Resolved(self.resolve(patch)),
            None => AutoReverbStatus::NoPatch,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
