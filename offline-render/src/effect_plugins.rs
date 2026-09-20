//! instrument の後段に挿す effect plugin の catalog と、ロード済み `PluginEntry` の表。
//!
//! # なぜ instrument の entry 表と別か
//! instrument の entry は「MML が指す音色」で引き分ける（[`crate::PluginEntries`]）が、
//! effect は MML 先頭 JSON の chain 要素のキーで決まり、catalog も preset の置き場も
//! 別物（`cmrt_core::AudioEffectCatalog`）。混在させると「音色無指定なら先頭」の規則が
//! effect にも効いてしまう。
//!
//! # ロードのタイミング
//! catalog の走査（preset ファイルの読み取り）も effect plugin の DLL ロードも、最初に
//! 要るときまで遅らせる。TUI 起動時に effect が無い環境で待たされないためで、
//! 一度読んだものは保持してレンダリングのたびに DLL を読み直さない。

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::Result;
use clack_host::prelude::PluginEntry;
use cmrt_core::{AudioEffectCatalog, AudioEffectPluginInfo, PluginKey, RenderEffects};

/// effect の catalog と entry を、レンダリング経路のあいだで共有する手元。
#[derive(Clone, Default)]
pub struct EffectPlugins {
    inner: Arc<EffectPluginsInner>,
}

#[derive(Default)]
enum EffectPluginsInner {
    /// chain 付きの MML を受け付けない経路（render server backend / テスト）。
    #[default]
    Disabled,
    Enabled {
        catalog: OnceLock<AudioEffectCatalog>,
        entries: Mutex<BTreeMap<PluginKey, PluginEntry>>,
    },
}

impl EffectPlugins {
    /// 組み込み既定パスから探す。走査は最初に catalog が要るときまで遅らせる。
    pub fn discover() -> Self {
        Self::enabled(OnceLock::new())
    }

    /// 組み立て済みの catalog から作る。catalog を手で並べたいテスト用でもある。
    pub fn with_catalog(catalog: AudioEffectCatalog) -> Self {
        let cell = OnceLock::new();
        let _ = cell.set(catalog);
        Self::enabled(cell)
    }

    /// chain 付きの MML を受け付けない経路（render server backend / テスト）用。
    pub fn none() -> Self {
        Self::default()
    }

    fn enabled(catalog: OnceLock<AudioEffectCatalog>) -> Self {
        Self {
            inner: Arc::new(EffectPluginsInner::Enabled {
                catalog,
                entries: Mutex::new(BTreeMap::new()),
            }),
        }
    }

    /// effect の catalog。受け付けない経路では `None`。初回は走査を行う。
    pub fn catalog(&self) -> Option<&AudioEffectCatalog> {
        match self.inner.as_ref() {
            EffectPluginsInner::Disabled => None,
            EffectPluginsInner::Enabled { catalog, .. } => {
                Some(catalog.get_or_init(AudioEffectCatalog::discover))
            }
        }
    }

    /// この経路が chain 付きの MML をレンダリングできるか。
    pub fn is_available(&self) -> bool {
        !matches!(self.inner.as_ref(), EffectPluginsInner::Disabled)
    }

    /// レンダリング 1 回ぶんの `RenderEffects` を組んで `render` に渡す。
    ///
    /// `RenderEffects` は catalog と entry loader への参照を持つだけなので、
    /// この呼び出しの中でしか使えない。
    pub fn with_render_effects<R>(&self, render: impl FnOnce(RenderEffects<'_>) -> R) -> R {
        let Some(catalog) = self.catalog() else {
            return render(RenderEffects::unsupported());
        };
        let load_entry = |plugin: &AudioEffectPluginInfo| self.entry(plugin);
        render(RenderEffects::new(catalog, &load_entry))
    }

    /// effect plugin の entry。初回にロードし、以後は保持したものを返す。
    fn entry(&self, plugin: &AudioEffectPluginInfo) -> Result<PluginEntry> {
        let EffectPluginsInner::Enabled { entries, .. } = self.inner.as_ref() else {
            anyhow::bail!("この render 経路は effect plugin をロードしない");
        };
        let mut entries = entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(entry) = entries.get(&plugin.key) {
            return Ok(entry.clone());
        }
        let entry = cmrt_core::load_entry(&plugin.plugin_path)?;
        entries.insert(plugin.key.clone(), entry.clone());
        Ok(entry)
    }
}

#[cfg(test)]
mod tests;
