//! 設定から「どのディレクトリに音色があるか」を導く。
//!
//! 畳み込みの規則そのものは play server repo 側の [`cmrt_server_config`] が単一ソース。
//! ここは [`Config`] を受け取る形へ合わせるだけの薄い層で、呼び出し側（app・tui-core・
//! daw など）のシグネチャを変えないために残している。
//!
//! ここから `CoreConfig` を組む処理は core-lib 側（`cmrt_core::core_config_from_config`）に
//! 置いてある。この crate を config 専用の葉 crate に保つため。

pub use cmrt_server_config::shared_patch_root_dir;

use std::path::Path;

use crate::{layered_patch_role_filters, Config, PatchRoles, PluginProfile};

pub fn configured_patch_dirs(cfg: &Config) -> Vec<String> {
    cmrt_server_config::configured_patch_dirs(cfg.patches_dirs.as_deref())
}

pub fn core_config_patch_root_dir(cfg: &Config) -> Option<String> {
    cmrt_server_config::patch_root_dir(cfg.patches_dirs.as_deref())
}

/// カタログに載せるプラグイン 1 つぶん。
///
/// 「どの音色置き場を、どの基点で相対化して、どの用途別絞り込みで扱うか」は
/// プラグインごとに違う。混在カタログではこれが複数並ぶので、1 つにまとめて持つ。
///
/// display 文字列（patch の表示パス）は `base` からの相対パスで作る。この文字列は
/// 保存済みの MML 先頭 JSON / history / DAW セル / grid session が指す**永続 ID** なので、
/// 基点が変わると保存済みデータが一斉に指し先を失う。
///
/// **プラグインを跨いで共通の親を取ってはいけない。** プラグインごとに音色置き場は
/// 無関係なツリーにあり（Surge XT は `C:\ProgramData\Surge XT` 配下、Dexed は
/// `%APPDATA%` 配下）、束ねると共通の親が `C:\` まで登る。display が
/// `ProgramData/Surge XT/patches_factory/...` のような形へ変わり、用途別カテゴリの
/// 絞り込みも保存済みデータも同時に壊れる（`docs/adr/0006-per-profile-relative-base.md`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogPlugin {
    /// プロファイル名。診断表示にしか使わない。
    pub name: String,
    pub plugin_path: String,
    pub plugin_id: Option<String>,
    /// このプラグインの音色の display 文字列を作る基点。
    /// 共通の親が取れないときは `None`（＝相対化せず絶対パスをそのまま display にする）。
    pub base: Option<String>,
    /// このプラグインの音色置き場。
    pub dirs: Vec<String>,
    /// このプラグインの音色に当てる用途別絞り込み（解決済み）。
    pub patch_roles: PatchRoles,
}

impl CatalogPlugin {
    /// このプラグインが Surge XT か。voicing 判定のデータ源の切り替えに使う。
    pub fn is_surge_xt(&self) -> bool {
        crate::is_surge_xt_plugin(self.plugin_id.as_deref(), &self.plugin_path)
    }

    /// このプラグインが Vaporizer2 か。
    ///
    /// 用途別カテゴリの組み込み既定（[`crate::PatchRoles::builtin_for`]）と、
    /// voicing 判定の方針の切り替えに使う。`.vvp` の mono/poly は音色ファイルの
    /// 先頭に書いてあるので、Surge の共有 JSON にも実行時 probe にも頼らない。
    pub fn is_vaporizer2(&self) -> bool {
        crate::is_vaporizer2_plugin(self.plugin_id.as_deref(), &self.plugin_path)
    }
}

/// カタログに音色を載せるプラグインの一覧。**先頭が既定プラグイン**
/// （＝音色を無指定にした行が鳴るもの）。
///
/// 先頭は `active_plugin` の解決結果（トップレベルへ焼き込み済み）。そのうしろへ、
/// **このマシンに実際にインストールされていて音色置き場も実在する**プロファイルを
/// 並べる。config への opt-in は要らない（`docs/adr/0005-mixed-catalog-on-by-default.md`）。
///
/// これが複数返すと、カタログ収集・用途別絞り込み・voicing 判定・オフライン
/// レンダリングの entry がまとめて混在対応になる。並びは決まった順（組み込み
/// プロファイル名の昇順 → config で足した名前）なので、`PluginEntries` のような
/// 「添字で対応づける表」と揃う。
pub fn catalog_plugins(cfg: &Config) -> Vec<CatalogPlugin> {
    catalog_plugins_with(cfg, installed_plugin_profiles(cfg))
}

/// [`catalog_plugins`] の組み立て部分。実在チェックを済ませたプロファイルを受け取る。
///
/// 「何がインストールされているか」を外から渡せる形にしてあるのは、テストが
/// 開発機のインストール状況に左右されないようにするため。
fn catalog_plugins_with(
    cfg: &Config,
    installed: Vec<(String, PluginProfile)>,
) -> Vec<CatalogPlugin> {
    let mut plugins = vec![active_catalog_plugin(cfg)];
    for (name, profile) in installed {
        let dirs = cmrt_server_config::configured_patch_dirs(profile.patches_dirs.as_deref());
        if dirs.is_empty() {
            continue;
        }
        let plugin = CatalogPlugin {
            patch_roles: PatchRoles::resolve(
                &profile.patch_roles,
                &PatchRoles::builtin_for(profile.plugin_id.as_deref(), &profile.plugin_path),
            ),
            name,
            plugin_path: profile.plugin_path,
            plugin_id: profile.plugin_id,
            base: shared_patch_root_dir(&dirs),
            dirs,
        };
        if let Some(listed) = plugins
            .iter_mut()
            .find(|listed| is_same_plugin(listed, &plugin))
        {
            // 既定プラグインと同じものが `[plugins.*]` にも書かれている。音色置き場は
            // 既定側（トップレベル or `active_plugin` の解決結果）が正なので捨てるが、
            // **用途別絞り込みだけは拾う**。`active_plugin` を書かない config では
            // `apply_active_plugin_profile` が動かないので、ここで拾わないと
            // `[plugins."Surge XT"]` に書いたカテゴリが黙って無視される。
            listed.patch_roles = PatchRoles::resolve_for_default_plugin(
                cfg,
                &layered_patch_role_filters(&cfg.active_patch_roles, &profile.patch_roles),
            );
            continue;
        }
        plugins.push(plugin);
    }
    plugins
}

/// 既定プラグイン（音色を無指定にした行が鳴るもの）ぶんのカタログ項目。
///
/// **ここだけは音色置き場の実在チェックをしない。** 設定に書かれた dir が無いことは
/// 設定ミスなので、一覧の収集がエラーになるという今までどおりの振る舞いを残す
/// （`docs/adr/0005-mixed-catalog-on-by-default.md`）。
fn active_catalog_plugin(cfg: &Config) -> CatalogPlugin {
    let dirs = configured_patch_dirs(cfg);
    CatalogPlugin {
        name: cfg
            .active_plugin
            .clone()
            .unwrap_or_else(|| crate::plugin_file_stem(&cfg.plugin_path).to_string()),
        plugin_path: cfg.plugin_path.clone(),
        plugin_id: cfg.plugin_id.clone(),
        base: shared_patch_root_dir(&dirs),
        dirs,
        patch_roles: PatchRoles::resolve_for_default_plugin(cfg, &cfg.active_patch_roles),
    }
}

/// このマシンで実際に使えるプラグインのプロファイル。**実在チェック済み**。
///
/// 「組み込み + config の `[plugins.*]` を合成し、表記ゆれを吸収し、プラグイン本体が
/// 実在するものだけに絞る」までは [`cmrt_server_config::installed_plugin_profiles`] が
/// 単一ソース。TUI 側で足しているのは**音色置き場の絞り込み**だけ。
///
/// - 音色置き場は**実在する dir だけ**に絞る。未インストールのプラグインの既定 dir で
///   `read_dir` が `Err` になり一覧全体が失敗する事故を避ける
///   （`docs/adr/0005-mixed-catalog-on-by-default.md`）。サーバー側は音色置き場を実在チェックの材料に
///   しない（インスタンスを作れるかだけを見る）ので、ここは TUI 固有
///
/// 並びはプロファイル名の昇順（`BTreeMap` の順）。`PluginEntries` のような
/// 「添字で対応づける表」と揃えるため、決まった順であることだけが要件。
fn installed_plugin_profiles(cfg: &Config) -> Vec<(String, PluginProfile)> {
    // 既定プラグインが定まらない config では混在させない。`plugin_path` が空なのは
    // 「どのプラグインも指していない」ということで、そもそも entry をロードできない
    // （読み手が「空です」と弾く）。同定できない以上、既定プラグインと同じものを
    // 二重に載せていないかも確かめられない。
    if cfg.plugin_path.trim().is_empty() {
        return Vec::new();
    }
    cmrt_server_config::installed_plugin_profiles(&cfg.plugins)
        .into_iter()
        .map(|(name, profile)| {
            let dirs: Vec<String> =
                cmrt_server_config::configured_patch_dirs(profile.patches_dirs.as_deref())
                    .into_iter()
                    .filter(|dir| Path::new(dir).is_dir())
                    .collect();
            (
                name,
                PluginProfile {
                    patches_dirs: Some(dirs),
                    ..profile
                },
            )
        })
        .collect()
}

/// 同じプラグインを 2 度カタログへ載せないための同定。
///
/// 既定プラグインはトップレベルへ焼き込み済みで、由来のプロファイル名が残っていない。
/// 名前では突き合わせられないので、プラグインそのものの同一性で見る。
///
/// `plugin_id` は `active_plugin` を使わない旧 config には書かれていないので、
/// **両方に書かれているときだけ** それで判定し、無ければ `plugin_path` のファイル名で
/// 見る（[`crate::is_surge_xt_plugin`] と同じ考え方）。これにより、既定プラグインの
/// `patches_dirs` を config で差し替えてあっても、組み込みプロファイルの既定 dir が
/// 二重に載ることはない。
fn is_same_plugin(a: &CatalogPlugin, b: &CatalogPlugin) -> bool {
    match (a.plugin_id.as_deref(), b.plugin_id.as_deref()) {
        (Some(a_id), Some(b_id)) => a_id == b_id,
        _ => crate::plugin_file_stem(&a.plugin_path)
            .eq_ignore_ascii_case(&crate::plugin_file_stem(&b.plugin_path)),
    }
}

#[cfg(test)]
mod tests;
