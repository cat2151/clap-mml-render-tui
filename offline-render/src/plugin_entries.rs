//! カタログのプラグインごとに、ロード済みの CLAP `PluginEntry` を引く表。
//!
//! # なぜ複数要るか
//! 混在カタログでは「この MML をどのプラグインで鳴らすか」が**音色ごとに変わる**
//! （`docs/adr/0001-patch-string-decides-the-plugin.md`）。in-process のオフラインレンダリングは
//! `PluginEntry` を直接使うので、鳴らしうるプラグインぶんのentryを共有状態へ公開し、
//! レンダリングのたびにpatch文字列で引き分ける。
//!
//! entry列を受け取る境界で[`cmrt_core::PluginKey`]へ変換し、レンダー中はcatalogの
//! 並び順ではなくkeyで参照する。先頭という規則は「音色無指定時の既定」を決める
//! 境界にだけ残す。

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use clack_host::prelude::PluginEntry;
use cmrt_runtime::CatalogPlugin;
use cmrt_tui_core::patch_plugins::PatchPlugins;

/// ロード済み `PluginEntry` への参照をカタログの並びで持つ表。
///
/// 実体を`Arc`で所有し、TUIのcache workerから一度だけ公開した後はrender worker間で
/// 共有する。render server backend / テストでは`Disabled`のままにする。
#[derive(Clone, Default)]
pub struct PluginEntries {
    inner: Arc<PluginEntriesInner>,
}

#[derive(Default)]
enum PluginEntriesInner {
    #[default]
    Disabled,
    Deferred(OnceLock<Result<LoadedPluginEntries, String>>),
}

pub(crate) struct LoadedPluginEntries {
    entries: Arc<[PluginEntry]>,
    entry_indices: BTreeMap<cmrt_core::PluginKey, usize>,
    plugin_indices: BTreeMap<cmrt_core::PluginKey, usize>,
    pub(crate) catalog: Vec<CatalogPlugin>,
    pub(crate) patch_plugins: PatchPlugins,
}

impl PluginEntries {
    /// `main()` がロードしたentry列から作る。
    ///
    /// **並びは `catalog_plugins(cfg)` と揃っていること。** ずれると別プラグインの
    /// entry で音色を開こうとして、Surge は 163 byte の SysEx を黙って無視する
    /// （`docs/adr/0009-offline-entry-map.md`）。
    pub fn from_loaded(catalog: Vec<CatalogPlugin>, entries: &[PluginEntry]) -> Self {
        let inner = OnceLock::new();
        let _ = inner.set(Ok(loaded_entries(catalog, entries.to_vec())));
        Self {
            inner: Arc::new(PluginEntriesInner::Deferred(inner)),
        }
    }

    /// TUI起動時には未完成で作り、cache workerから一度だけ完成させる。
    pub fn pending() -> Self {
        Self {
            inner: Arc::new(PluginEntriesInner::Deferred(OnceLock::new())),
        }
    }

    /// workerが所有するentryを共有状態へ一度だけ公開する。
    pub fn publish_owned(
        &self,
        catalog: Vec<CatalogPlugin>,
        entries: Vec<PluginEntry>,
    ) -> Result<(), String> {
        let PluginEntriesInner::Deferred(inner) = self.inner.as_ref() else {
            return Err("in-process offline renderは無効です".to_string());
        };
        inner
            .set(Ok(loaded_entries(catalog, entries)))
            .map_err(|_| "PluginEntryは既に初期化されています".to_string())
    }

    pub fn publish_error(&self, error: impl Into<String>) {
        if let PluginEntriesInner::Deferred(inner) = self.inner.as_ref() {
            let _ = inner.set(Err(error.into()));
        }
    }

    /// in-process レンダリングを使わない経路（render server backend / テスト）用。
    pub fn none() -> Self {
        Self::default()
    }

    /// in-process レンダリングができるか。既定プラグインの entry があるかで判断する。
    pub fn is_available(&self) -> bool {
        self.loaded()
            .ok()
            .is_some_and(|loaded| !loaded.entry_indices.is_empty())
    }

    pub(crate) fn entry(&self, key: &cmrt_core::PluginKey) -> Result<PluginEntry, anyhow::Error> {
        let loaded = self.loaded()?;
        let index = loaded.entry_indices.get(key).copied().ok_or_else(|| {
            anyhow::anyhow!("in-process offline render requires loaded PluginEntry: {key}")
        })?;
        loaded
            .entries
            .get(index)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("PluginEntry indexが範囲外です: {index}"))
    }

    pub(crate) fn loaded(&self) -> Result<&LoadedPluginEntries, anyhow::Error> {
        let PluginEntriesInner::Deferred(inner) = self.inner.as_ref() else {
            anyhow::bail!("in-process offline render requires loaded PluginEntry");
        };
        let result = inner
            .get()
            .ok_or_else(|| anyhow::anyhow!("patch catalog / PluginEntryを準備中です"))?;
        result
            .as_ref()
            .map_err(|error| anyhow::anyhow!(error.clone()))
    }
}

pub(crate) fn loaded_entries(
    catalog: Vec<CatalogPlugin>,
    entries: Vec<PluginEntry>,
) -> LoadedPluginEntries {
    let patch_plugins = PatchPlugins::from_catalog(catalog.clone());
    let plugin_indices = patch_plugins
        .plugins()
        .iter()
        .enumerate()
        .filter_map(|(index, _)| {
            patch_plugins
                .audio_info(index)
                .map(|plugin| (plugin.key.clone(), index))
        })
        .collect::<BTreeMap<_, _>>();
    let entry_indices = entries
        .iter()
        .enumerate()
        .filter_map(|(index, _)| {
            patch_plugins
                .audio_info(index)
                .map(|plugin| (plugin.key.clone(), index))
        })
        .collect();
    LoadedPluginEntries {
        entries: entries.into(),
        entry_indices,
        plugin_indices,
        patch_plugins,
        catalog,
    }
}

impl LoadedPluginEntries {
    pub(crate) fn plugin_index(&self, key: &cmrt_core::PluginKey) -> Option<usize> {
        self.plugin_indices.get(key).copied()
    }
}

#[cfg(test)]
mod tests;
