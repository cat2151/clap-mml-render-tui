//! 音色の自動抽選で、plugin ごとの出現率（`0` / `normal`）を持つ設定。
//!
//! 効くのは用途別候補（[`crate::GridSequencerScreen::patch_candidates_for_purpose`]）を
//! 通る抽選と PATCH 欄の wheel。patch selector で手で選ぶ一覧は絞らない。
//!
//! plugin は catalog のプロファイル名で指す。既定は全 plugin が `normal` なので、
//! `0` にした名前だけを持つ。catalog から消えた plugin の名前も残す（設定を戻せば効く）。

use std::collections::BTreeSet;

use crate::{log_line, GridSequencerContext, GridSequencerScreen};

/// 1 plugin の出現率。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginWeight {
    Zero,
    Normal,
}

impl PluginWeight {
    pub fn label(self) -> &'static str {
        match self {
            Self::Zero => "0",
            Self::Normal => "normal",
        }
    }
}

/// 出現率 `0` の plugin 名の集合。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PluginWeights {
    zero: BTreeSet<String>,
}

impl PluginWeights {
    pub fn from_zero_plugins(names: impl IntoIterator<Item = String>) -> Self {
        Self {
            zero: names.into_iter().collect(),
        }
    }

    /// セッションへ保存する、出現率 `0` の plugin 名。名前順。
    pub fn zero_plugins(&self) -> impl Iterator<Item = &str> {
        self.zero.iter().map(String::as_str)
    }

    pub fn weight(&self, plugin: &str) -> PluginWeight {
        if self.zero.contains(plugin) {
            PluginWeight::Zero
        } else {
            PluginWeight::Normal
        }
    }

    fn toggle(&mut self, plugin: &str) {
        if !self.zero.remove(plugin) {
            self.zero.insert(plugin.to_string());
        }
    }
}

impl GridSequencerScreen {
    pub fn plugin_weights(&self) -> &PluginWeights {
        &self.plugin_weights
    }

    /// overlay の plugin 行。`0` ↔ `normal` を切り替える。
    pub(crate) fn toggle_plugin_weight(&mut self, plugin: &str) {
        self.plugin_weights.toggle(plugin);
        // 抽選済みの次サイクルは変更前の候補から引いている。
        self.cancel_cycle_swap_preserving_drain();
        log_line(&format!(
            "grid-sequencer: plugin-weight {plugin}={}",
            self.plugin_weights.weight(plugin).label(),
        ));
    }

    /// この音色を自動抽選へ出すか。plugin を引けない音色は出す。
    pub(crate) fn drawable_patch(&self, patch: &str, ctx: &GridSequencerContext<'_>) -> bool {
        self.plugin_weights.zero.is_empty()
            || ctx
                .patch_plugin_name(patch)
                .is_none_or(|plugin| self.plugin_weights.weight(plugin) == PluginWeight::Normal)
    }
}

#[cfg(test)]
mod tests;
