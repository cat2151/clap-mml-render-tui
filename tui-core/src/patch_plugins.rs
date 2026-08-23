//! patch 文字列 1 本から、その音色を鳴らすカタログ上のプラグインを引く表。
//!
//! # なぜ要るか
//! カタログに複数プラグインの音色が並ぶと、「用途別カテゴリで絞る」「mono/poly を
//! どう判定する」といった答えが**音色ごとに変わる**。Surge のカテゴリを Dexed の
//! cartridge へ当てると候補が全滅し（cartridge にカテゴリ階層が無い）、逆に
//! 「全部 poly とみなす」を Surge へ当てると和音行へ mono の音色が来る。
//!
//! # 何で判別しているか
//! 材料は **patch 文字列の形だけ**（`.syx` / `.vvp` / `.floe-preset` / `.sfz` を含むか）。
//! patch 文字列そのものへプラグイン名を入れる仕様変更は、display 文字列が
//! 永続 ID であるため保存済みデータの移行が要る（`docs/adr/0001-patch-string-decides-the-plugin.md`）。
//! 「このプラグインはどちらの形を扱うか」は `cmrt_server_config::patch_form_of` が
//! 単一ソース（サーバー側の `kind_for_patch` も同じものを通す）。

// カタログ 1 プラグインぶんの型はここから再輸出する。画面 crate（grid sequencer など）は
// config crate に依存せず、この表を通してだけプラグイン別の設定に触れる。
pub use cmrt_runtime::{CatalogPlugin, PatchRoles};

use cmrt_runtime::{catalog_plugins, patch_form_of, Config, PatchForm};

pub struct PatchPlugins {
    /// カタログに音色を載せるプラグイン。先頭が既定プラグイン。
    plugins: Vec<CatalogPlugin>,
    /// patch 文字列の形ごとの、`plugins` の添字。
    state_file: usize,
    cartridge: usize,
    /// `.vvp`（Vaporizer2）。**`state_file` と分けて持つ。** 単位は「1 ファイル = 1 音色」で
    /// 同じだが、一緒にすると Surge の添字へ落ち、Surge のカテゴリで候補が全滅する。
    vvp: usize,
    /// `.floe-preset`（Floe）。Surge XT の state-file routing と混ぜないための添字。
    floe_preset: usize,
    /// `.sfz`（sforzando）。CLAP state 経路へ誤投入しないための添字。
    sfz: usize,
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
        Self::new(vec![CatalogPlugin {
            name: String::new(),
            plugin_path: String::new(),
            plugin_id: None,
            base: None,
            dirs: Vec::new(),
            resolved_patches: None,
            source_notices: Vec::new(),
            patch_roles,
        }])
    }

    /// 形ごとの引き当て先を決める。**既定プラグイン（先頭）を優先する。**
    ///
    /// その形を扱うプラグインがカタログに無ければ既定へ落とす。落とし先の絞り込みが
    /// 当たらなくても候補が減るだけで、鳴らせない音色が候補に出るよりは軽い。
    fn new(plugins: Vec<CatalogPlugin>) -> Self {
        let index_of = |wants: PatchForm| {
            plugins
                .iter()
                .position(|plugin| {
                    patch_form_of(plugin.plugin_id.as_deref(), &plugin.plugin_path) == wants
                })
                .unwrap_or(0)
        };
        Self {
            state_file: index_of(PatchForm::StateFile),
            cartridge: index_of(PatchForm::Cartridge),
            vvp: index_of(PatchForm::Vvp),
            floe_preset: index_of(PatchForm::FloePreset),
            sfz: index_of(PatchForm::Sfz),
            plugins,
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
    /// 判定の実体は play server repo 側（`is_cartridge_patch_path` / `is_vvp_patch_path`）が
    /// 単一ソース。サーバーの `kind_for_patch` も同じ関数を通すので、
    /// 「画面に出た音色が、送った先のインスタンスでは鳴らせない」がここでは起きない。
    pub fn index_for_patch(&self, patch: &str) -> usize {
        if cmrt_core::is_cartridge_patch_path(patch) {
            self.cartridge
        } else if cmrt_core::is_sfz_patch_path(patch) {
            self.sfz
        } else if cmrt_core::is_floe_preset_path(patch) {
            self.floe_preset
        } else if cmrt_core::is_vvp_patch_path(patch) {
            self.vvp
        } else {
            self.state_file
        }
    }

    /// この patch 文字列を鳴らすプラグイン。
    pub fn for_patch(&self, patch: &str) -> &CatalogPlugin {
        &self.plugins[self.index_for_patch(patch)]
    }

    /// カタログに Surge XT の音色が載りうるか。
    ///
    /// Surge 専用の共有 voicing JSON を取りに行くかどうかの判断に使う。
    pub fn any_surge_xt(&self) -> bool {
        self.plugins.iter().any(CatalogPlugin::is_surge_xt)
    }
}

#[cfg(test)]
mod tests;
