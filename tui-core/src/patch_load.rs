//! パッチ一覧のバックグラウンド読み込み状態（画面横断で共有）。
//!
//! 起動時の同期I/Oによる遅延を避けるため、永続cacheは別スレッドで読む。
//! その途中経過を notepad（音色選択）と keyboard（patch catalog）の双方が読むため、
//! 状態型だけをここに置く。スレッドの起動は所有者（app）が行う。

use std::sync::Arc;

use crate::patch_plugins::{CatalogPlugin, PatchPlugins};

/// file cacheから復元した、画面横断のpatch catalog一式。
#[derive(Clone)]
pub struct PatchCatalogSnapshot {
    pairs: Vec<(String, String)>,
    plugins: PatchPlugins,
    catalog_notes: Vec<String>,
}

impl PatchCatalogSnapshot {
    pub fn new(
        pairs: Vec<(String, String)>,
        plugins: Vec<CatalogPlugin>,
        catalog_notes: Vec<String>,
    ) -> Self {
        Self {
            pairs,
            plugins: PatchPlugins::from_catalog(plugins),
            catalog_notes,
        }
    }

    pub fn pairs(&self) -> &[(String, String)] {
        &self.pairs
    }

    pub fn patch_plugins(&self) -> &PatchPlugins {
        &self.plugins
    }

    pub fn catalog_notes(&self) -> &[String] {
        &self.catalog_notes
    }

    pub fn catalog_plugins(&self) -> &[CatalogPlugin] {
        self.plugins.plugins()
    }

    /// catalog metadataを必要としない画面テスト向け。
    pub fn from_pairs(pairs: Vec<(String, String)>) -> Self {
        Self::new(
            pairs,
            vec![CatalogPlugin {
                name: String::new(),
                plugin_path: String::new(),
                plugin_id: None,
                base: None,
                dirs: Vec::new(),
                resolved_patches: None,
                source_notices: Vec::new(),
                patch_roles: Default::default(),
            }],
            Vec::new(),
        )
    }
}

/// バックグラウンドpatch cache読み込みの状態。
pub enum PatchLoadState {
    Loading,
    Ready(Arc<PatchCatalogSnapshot>),
    Err(String),
}

impl PatchLoadState {
    pub fn ready(pairs: Vec<(String, String)>) -> Self {
        Self::Ready(Arc::new(PatchCatalogSnapshot::from_pairs(pairs)))
    }
}
