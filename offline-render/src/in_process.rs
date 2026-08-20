//! in-process の CLAP ホスト実行で、MML 1 本を**どのプラグインで鳴らすか**を引き当てる。
//!
//! # なぜ引き当てが要るか
//! カタログに複数プラグインの音色が並ぶと、「この MML をどのプラグインへ渡すか」は
//! 音色ごとに変わる（`docs/adr/0001-patch-string-decides-the-plugin.md`）。判別材料は MML 先頭 JSON の
//! patch 文字列の形だけで、判別規則はサーバー側と同じものを通す。
//!
//! # 間違えるとどうなるか
//! Surge のインスタンスへ DX7 の cartridge を送ると、Surge は理解できない 163 byte を
//! **黙って無視する。エラーにならない**（同 §9）。つまり「音色を変えたのに前の音のまま、
//! 操作は成功扱い」という静かな間違いになるので、引き当ては 1 か所へ寄せる。

use anyhow::Result;
use clack_host::prelude::PluginEntry;
use cmrt_core::{core_config_for_plugin, embedded_patch_ref, CoreConfig};
use cmrt_runtime::{catalog_plugins, CatalogPlugin, Config};
use cmrt_tui_core::patch_plugins::PatchPlugins;

use crate::PluginEntries;

/// in-process レンダリングで使う、カタログのプラグインごとの資材。
pub struct InProcessPlugins {
    /// patch 文字列 → カタログ上の添字。
    patch_plugins: PatchPlugins,
    /// `catalog_plugins(cfg)` と同じ並び。空にはならない（先頭は必ず既定プラグイン）。
    slots: Vec<InProcessSlot>,
}

/// カタログ 1 プラグインぶんの、ロード済み entry とレンダリング設定。
struct InProcessSlot {
    entry_ptr: usize,
    core_cfg: CoreConfig,
}

impl InProcessPlugins {
    pub fn new(cfg: &Config, entries: &PluginEntries) -> Self {
        Self::from_catalog(cfg, catalog_plugins(cfg), entries)
    }

    /// 組み立て済みのカタログから作る。カタログを手で並べたいテスト用でもある。
    fn from_catalog(cfg: &Config, catalog: Vec<CatalogPlugin>, entries: &PluginEntries) -> Self {
        let slots = catalog
            .iter()
            .enumerate()
            .map(|(index, plugin)| InProcessSlot {
                entry_ptr: entries.ptr(index),
                core_cfg: core_config_for_plugin(cfg, plugin),
            })
            .collect();
        Self {
            patch_plugins: PatchPlugins::from_catalog(catalog),
            slots,
        }
    }

    /// この MML を鳴らすプラグインの entry とレンダリング設定。
    ///
    /// `CoreConfig` の `plugin_id` と `patches_dir`（音色パスの解決基点）は
    /// プラグインごとに違うので、entry だけ差し替えても足りない。必ず対で使うこと。
    pub fn for_mml(&self, mml: &str) -> Result<(&'static PluginEntry, &CoreConfig)> {
        let slot = self.slot(self.index_for_mml(mml));
        Ok((plugin_entry(slot.entry_ptr)?, &slot.core_cfg))
    }

    /// この MML を鳴らすプラグインの、カタログ上の添字。
    ///
    /// **音色を無指定にした MML は常に既定プラグイン（先頭）**
    /// （`docs/adr/0004-default-plugin-owns-unspecified-patches.md`）。patch 文字列の形で引くと、
    /// 空文字列が「cartridge ではない」と判定されて、
    /// 既定が Dexed のときに無指定の MML まで Surge 側へ飛ぶ。
    pub(crate) fn index_for_mml(&self, mml: &str) -> usize {
        match embedded_patch_ref(mml) {
            Some(patch) => self.patch_plugins.index_for_patch(&patch),
            None => 0,
        }
    }

    /// カタログ上 `index` 番目のプラグインのレンダリング設定。範囲外は既定プラグインへ落とす。
    pub(crate) fn core_cfg(&self, index: usize) -> &CoreConfig {
        &self.slot(index).core_cfg
    }

    /// カタログ上 `index` 番目のプラグインの entry。範囲外は既定プラグインへ落とす。
    pub(crate) fn entry(&self, index: usize) -> Result<&'static PluginEntry> {
        plugin_entry(self.slot(index).entry_ptr)
    }

    fn slot(&self, index: usize) -> &InProcessSlot {
        self.slots.get(index).unwrap_or(&self.slots[0])
    }
}

fn plugin_entry(entry_ptr: usize) -> Result<&'static PluginEntry> {
    if entry_ptr == 0 {
        anyhow::bail!("in-process offline render requires a loaded CLAP PluginEntry");
    }
    // SAFETY: production callers pass a pointer to the PluginEntry owned by main(), and
    // existing render workers already rely on that entry outliving the worker threads.
    Ok(unsafe { &*(entry_ptr as *const PluginEntry) })
}

#[cfg(test)]
mod tests;
