//! patch 文字列 1 本から、その音色を鳴らすカタログ上のプラグインを引く表。
//!
//! # なぜ要るか
//! カタログに複数プラグインの音色が並ぶと、「用途別カテゴリで絞る」「mono/poly を
//! どう判定する」といった答えが**音色ごとに変わる**。Surge のカテゴリを Dexed の
//! cartridge へ当てると候補が全滅し（cartridge にカテゴリ階層が無い）、逆に
//! 「全部 poly とみなす」を Surge へ当てると和音行へ mono の音色が来る。
//!
//! Plugin-specific routing is owned by the play-server shared core.  This
//! module only associates the returned [`cmrt_core::PluginKey`] with TUI
//! configuration such as resolved role filters.

// カタログ 1 プラグインぶんの型はここから再輸出する。画面 crate（grid sequencer など）は
// config crate に依存せず、この表を通してだけプラグイン別の設定に触れる。
pub use cmrt_runtime::{CatalogPlugin, PatchRoles};

use cmrt_runtime::{catalog_plugins, Config};

#[derive(Clone)]
pub struct PatchPlugins {
    /// カタログに音色を載せるプラグイン。先頭が既定プラグイン。
    plugins: Vec<CatalogPlugin>,
    routing: cmrt_core::AudioPluginCatalog,
    single_fallback: bool,
}

impl PatchPlugins {
    pub fn from_config(cfg: &Config) -> Self {
        Self::new(catalog_plugins(cfg))
    }

    /// 組み立て済みのカタログから作る。config を通さない呼び出し側（画面テスト）用。
    pub fn from_catalog(plugins: Vec<CatalogPlugin>) -> Self {
        Self::new(plugins)
    }

    /// プラグイン 1 つだけのカタログ。用途別絞り込みだけを差し替えたいとき用。
    pub fn single_plugin(patch_roles: PatchRoles) -> Self {
        let mut plugins = Self::new(vec![CatalogPlugin {
            name: String::new(),
            plugin_path: String::new(),
            plugin_id: None,
            base: None,
            dirs: Vec::new(),
            resolved_patches: None,
            source_notices: Vec::new(),
            patch_roles,
        }]);
        plugins.single_fallback = true;
        plugins
    }

    fn new(plugins: Vec<CatalogPlugin>) -> Self {
        let routing = cmrt_core::AudioPluginCatalog::new(
            plugins
                .iter()
                .map(|plugin| {
                    cmrt_core::AudioPluginInfo::new(
                        plugin.name.clone(),
                        plugin.plugin_path.clone(),
                        plugin.plugin_id.clone(),
                        plugin.base.clone(),
                    )
                })
                .collect(),
        );
        Self {
            plugins,
            routing,
            single_fallback: false,
        }
    }

    /// カタログに音色を載せるプラグイン。先頭が既定プラグイン。
    pub fn plugins(&self) -> &[CatalogPlugin] {
        &self.plugins
    }

    /// この patch 文字列を鳴らすプラグインの、[`PatchPlugins::plugins`] 上の添字。
    ///
    /// patch 一覧の絞り込みは patch ごとにこれを引くので、プラグインごとの材料は
    /// 呼び出し側が**添字で引ける形に組んでから**ループへ入ること。
    /// 判定の実体はplay server shared coreが単一ソース。サーバーの実行経路も同じ
    /// 判定関数を通すので、
    /// 「画面に出た音色が、送った先のインスタンスでは鳴らせない」がここでは起きない。
    pub fn index_for_patch(&self, patch: &str) -> Result<usize, cmrt_core::RouteError> {
        if self.single_fallback {
            return Ok(0);
        }
        let routed = self.routing.route_patch(patch)?;
        self.routing
            .plugins()
            .iter()
            .position(|plugin| plugin.key == routed.key)
            .ok_or_else(|| cmrt_core::RouteError::PluginMissing {
                key: routed.key.clone(),
            })
    }

    /// この patch 文字列を鳴らすプラグイン。
    pub fn for_patch(&self, patch: &str) -> Result<&CatalogPlugin, cmrt_core::RouteError> {
        Ok(&self.plugins[self.index_for_patch(patch)?])
    }

    pub fn patch_ref(&self, patch: &str) -> Result<cmrt_core::PatchRef, cmrt_core::RouteError> {
        let plugin = self.routing.route_patch(patch)?;
        Ok(cmrt_core::PatchRef {
            plugin: plugin.key.clone(),
            display: patch.to_string(),
        })
    }

    /// Keyed patch metadataから、対応する抽象plugin情報を引く。
    pub fn audio_info_for_ref(
        &self,
        patch: &cmrt_core::PatchRef,
    ) -> Result<&cmrt_core::AudioPluginInfo, cmrt_core::RouteError> {
        self.routing.route_ref(patch)
    }

    /// Whether any catalog entry delegates voicing to the external lookup
    /// layers owned by the application.
    pub fn any_external_voicing(&self) -> bool {
        self.routing
            .plugins()
            .iter()
            .any(|plugin| plugin.voicing_source() == cmrt_core::PluginVoicingSource::ExternalLookup)
    }

    pub fn audio_info(&self, index: usize) -> Option<&cmrt_core::AudioPluginInfo> {
        self.routing.plugins().get(index)
    }
}

#[cfg(test)]
mod tests;
