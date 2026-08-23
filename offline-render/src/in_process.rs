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
#[cfg(test)]
use cmrt_runtime::CatalogPlugin;
use cmrt_runtime::Config;

use crate::PluginEntries;

/// in-process レンダリングで使う、カタログのプラグインごとの資材。
pub struct InProcessPlugins {
    cfg: Config,
    entries: PluginEntries,
    #[cfg(test)]
    catalog_override: Option<crate::plugin_entries::LoadedPluginEntries>,
}

impl InProcessPlugins {
    pub fn new(cfg: &Config, entries: &PluginEntries) -> Self {
        Self {
            cfg: cfg.clone(),
            entries: entries.clone(),
            #[cfg(test)]
            catalog_override: None,
        }
    }

    /// 組み立て済みのカタログから作る。カタログを手で並べたいテスト用でもある。
    #[cfg(test)]
    fn from_catalog(cfg: &Config, catalog: Vec<CatalogPlugin>, entries: &PluginEntries) -> Self {
        Self {
            cfg: cfg.clone(),
            entries: entries.clone(),
            catalog_override: Some(crate::plugin_entries::loaded_entries(catalog, Vec::new())),
        }
    }

    /// この MML を鳴らすプラグインの entry とレンダリング設定。
    ///
    /// `CoreConfig` の `plugin_id` と `patches_dir`（音色パスの解決基点）は
    /// プラグインごとに違うので、entry だけ差し替えても足りない。必ず対で使うこと。
    pub fn for_mml(&self, mml: &str) -> Result<(PluginEntry, CoreConfig)> {
        let key = self.plugin_key_for_mml(mml)?;
        let core_cfg = self.core_cfg_for_key(&key)?;
        Ok((self.entries.entry(&key)?, core_cfg))
    }

    pub(crate) fn plugin_key_for_mml(&self, mml: &str) -> Result<cmrt_core::PluginKey> {
        let loaded = self.loaded_catalog()?;
        match embedded_patch_ref(mml) {
            Some(patch) => loaded
                .patch_plugins
                .patch_ref(&patch)
                .map(|patch| patch.plugin)
                .map_err(anyhow::Error::new),
            None => loaded
                .patch_plugins
                .audio_info(0)
                .map(|plugin| plugin.key.clone())
                .ok_or_else(|| anyhow::anyhow!("既定プラグインがcatalogにありません")),
        }
    }

    /// この MML を鳴らすプラグインの、カタログ上の添字。旧来の期待値を確認するテスト用。
    ///
    /// **音色を無指定にした MML は常に既定プラグイン（先頭）**
    /// （`docs/adr/0004-default-plugin-owns-unspecified-patches.md`）。patch 文字列の形で引くと、
    /// 空文字列が「cartridge ではない」と判定されて、
    /// 既定が Dexed のときに無指定の MML まで Surge 側へ飛ぶ。
    #[cfg(test)]
    pub(crate) fn index_for_mml(&self, mml: &str) -> Result<usize> {
        let loaded = self.loaded_catalog()?;
        let key = self.plugin_key_for_mml(mml)?;
        loaded
            .plugin_index(&key)
            .ok_or_else(|| anyhow::anyhow!("plugin keyがcatalogにありません: {key}"))
    }

    /// カタログ上 `index` 番目のプラグインのレンダリング設定。
    pub(crate) fn core_cfg(&self, index: usize) -> Result<CoreConfig> {
        let loaded = self.loaded_catalog()?;
        let plugin = loaded
            .catalog
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("plugin indexがcatalog範囲外です: {index}"))?;
        Ok(core_config_for_plugin(&self.cfg, plugin))
    }

    pub(crate) fn core_cfg_for_key(&self, key: &cmrt_core::PluginKey) -> Result<CoreConfig> {
        let loaded = self.loaded_catalog()?;
        let index = loaded
            .plugin_index(key)
            .ok_or_else(|| anyhow::anyhow!("plugin keyがcatalogにありません: {key}"))?;
        self.core_cfg(index)
    }

    /// カタログ上 `index` 番目のプラグインの entry。
    pub(crate) fn entry(&self, key: &cmrt_core::PluginKey) -> Result<PluginEntry> {
        let _ = self.loaded_catalog()?;
        self.entries.entry(key)
    }

    fn loaded_catalog(&self) -> Result<&crate::plugin_entries::LoadedPluginEntries> {
        #[cfg(test)]
        if let Some(catalog) = &self.catalog_override {
            return Ok(catalog);
        }
        self.entries.loaded()
    }
}

#[cfg(test)]
mod tests;
