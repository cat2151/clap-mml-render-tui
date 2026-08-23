//! 設定から「どのディレクトリに音色があるか」を導く。
//!
//! 畳み込みの規則そのものは play server repo 側の [`cmrt_server_config`] が単一ソース。
//! ここは [`Config`] を受け取る形へ合わせるだけの薄い層で、呼び出し側（app・tui-core・
//! daw など）のシグネチャを変えないために残している。
//!
//! ここから `CoreConfig` を組む処理は core-lib 側（`cmrt_core::core_config_from_config`）に
//! 置いてある。この crate を config 専用の葉 crate に保つため。
//!
//! 診断は戻り値の `source_notices` / `CatalogSkipReason` として返す。この library から
//! stdout/stderr へ直接書くと alternate screen 中の TUI 描画を壊すため、表示とログ保存は
//! app 側の overlay / 注入済み log sink に任せる。

pub use cmrt_server_config::shared_patch_root_dir;

use std::path::PathBuf;

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
    /// Adapter が「実際にロード可能」と解決済みの file。`None` は通常の directory scan。
    /// vendor metadata は play-server 側に残し、TUI へは path だけを渡す。
    pub resolved_patches: Option<Vec<PathBuf>>,
    /// 一部 source の破損や、ロード不能 file の除外を知らせる汎用診断。
    pub source_notices: Vec<String>,
    /// このプラグインの音色に当てる用途別絞り込み（解決済み）。
    pub patch_roles: PatchRoles,
}

/// カタログへ載せなかったプラグインと、その理由。
///
/// **黙って外す**のが今までの倒れ方で、それ自体は変えない（音色置き場が無いプラグインを
/// 載せると、別プラグインの音色がそのインスタンスへ送られる。
/// `docs/adr/0005-mixed-catalog-on-by-default.md`）。変えたのは
/// 「外したことが誰にも見えない」ほうで、CLI 診断・ログ・音色選択の 3 経路が
/// これを読んで同じ 1 行を出す。
///
/// インストールされていないプラグインはここに出てこない。
/// [`installed_plugin_profiles`] の時点で落ちており、「入れていないものが出ない」のは
/// 説明の要らない当たり前だから。ここに出るのは**入っているのに設定不足で外れたもの**だけ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedCatalogPlugin {
    /// プロファイル名。config の `[plugins.<名前>]` と同じ綴り。
    pub name: String,
    pub reason: CatalogSkipReason,
}

/// カタログから外した理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogSkipReason {
    /// `patches_dirs` が書かれていない。**Vaporizer2 の組み込みプロファイルがこれ**
    /// （プリセット置き場がインストールごとに違うので既定値を持たない）。
    NoPatchDirs,
    /// `patches_dirs` は書かれているが、1 つも実在しない。
    /// 書いた dir を持って回るのは、綴り間違いを名指しで返すため。
    PatchDirsMissing(Vec<String>),
    /// Plugin adapter がロード可能な patch source を解決できなかった。
    PatchSourceUnavailable {
        configured_missing: Vec<String>,
        source_error: String,
    },
}

impl SkippedCatalogPlugin {
    /// CLI 診断・ログ・音色選択の注記が共有する 1 行。
    ///
    /// **文言をここ以外に持たない。** 3 経路で書き分けると、直すときに片方だけ古くなる。
    pub fn notice_line(&self) -> String {
        match &self.reason {
            CatalogSkipReason::NoPatchDirs => format!(
                "{} は config.toml の [plugins.{}] に patches_dirs が無いため一覧に出ません",
                self.name, self.name
            ),
            CatalogSkipReason::PatchDirsMissing(dirs) => format!(
                "{} は [plugins.{}] の patches_dirs が実在しないため一覧に出ません: {}",
                self.name,
                self.name,
                dirs.join(" / ")
            ),
            CatalogSkipReason::PatchSourceUnavailable {
                configured_missing,
                source_error,
            } => {
                let configured = if configured_missing.is_empty() {
                    "未設定".to_string()
                } else {
                    format!("実在しない: {}", configured_missing.join(" / "))
                };
                format!(
                    "{} はロード可能な音色 source が無いため一覧に出ません: config {configured} / resolver {source_error}",
                    self.name
                )
            }
        }
    }

    /// ログ行に書く機械可読な理由。`key=value` のログを grep するためのもの。
    pub fn reason_code(&self) -> &'static str {
        match self.reason {
            CatalogSkipReason::NoPatchDirs => "no-patches-dirs",
            CatalogSkipReason::PatchDirsMissing(_) => "patch-dirs-missing",
            CatalogSkipReason::PatchSourceUnavailable { .. } => "patch-source-unavailable",
        }
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
    catalog_plugins_detailed(cfg).0
}

/// [`catalog_plugins`] と、**そのとき外したプラグインの一覧**。
///
/// 載せたぶんと外したぶんを 1 回の走査で同時に返す。外れた判定を別の関数で
/// 書き直すと、条件がずれて「一覧には出ないのに『外していません』と言う」状態になる。
pub fn catalog_plugins_detailed(cfg: &Config) -> (Vec<CatalogPlugin>, Vec<SkippedCatalogPlugin>) {
    catalog_plugins_with(cfg, installed_plugin_profiles(cfg))
}

/// カタログから外したプラグインだけが要るとき用。
pub fn skipped_catalog_plugins(cfg: &Config) -> Vec<SkippedCatalogPlugin> {
    catalog_plugins_detailed(cfg).1
}

/// 画面へ出す「一覧に出てこない音色がある」ことの案内。外れたものが無ければ空。
///
/// 音色選択を持つ画面はどれもこれを使う。**組み立てをここ 1 か所に置く**のは、
/// 画面ごとに数え方が分かれると「ある画面にだけ案内が出ない」からで、それは
/// 案内が無いこと自体が症状なので誰も気づけない。
pub fn catalog_notice_lines(cfg: &Config) -> Vec<String> {
    let (plugins, skipped) = catalog_plugins_detailed(cfg);
    plugins
        .iter()
        .flat_map(|plugin| {
            plugin
                .source_notices
                .iter()
                .map(move |notice| format!("{}: {notice}", plugin.name))
        })
        .chain(skipped.iter().map(SkippedCatalogPlugin::notice_line))
        .collect()
}

/// [`catalog_plugins`] の組み立て部分。実在チェックを済ませたプロファイルを受け取る。
///
/// 「何がインストールされているか」を外から渡せる形にしてあるのは、テストが
/// 開発機のインストール状況に左右されないようにするため。
fn catalog_plugins_with(
    cfg: &Config,
    installed: Vec<InstalledProfile>,
) -> (Vec<CatalogPlugin>, Vec<SkippedCatalogPlugin>) {
    let mut plugins = vec![active_catalog_plugin(cfg)];
    let mut skipped = Vec::new();
    for InstalledProfile {
        name,
        profile,
        missing_dirs,
        resolved_patches,
        source_notices,
        source_error,
    } in installed
    {
        let dirs = cmrt_server_config::configured_patch_dirs(profile.patches_dirs.as_deref());
        if let Some(source_error) = source_error {
            skipped.push(SkippedCatalogPlugin {
                name,
                reason: CatalogSkipReason::PatchSourceUnavailable {
                    configured_missing: missing_dirs,
                    source_error,
                },
            });
            continue;
        }
        if dirs.is_empty() {
            // 外した事実をここで作る。判定と理由が同じ 1 か所に居ることが要点。
            let reason = if missing_dirs.is_empty() {
                CatalogSkipReason::NoPatchDirs
            } else {
                CatalogSkipReason::PatchDirsMissing(missing_dirs)
            };
            skipped.push(SkippedCatalogPlugin { name, reason });
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
            resolved_patches,
            source_notices,
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
    (plugins, skipped)
}

/// 既定プラグイン（音色を無指定にした行が鳴るもの）ぶんのカタログ項目。
///
/// **ここだけは音色置き場の実在チェックをしない。** 設定に書かれた dir が無いことは
/// 設定ミスなので、一覧の収集がエラーになるという今までどおりの振る舞いを残す
/// （`docs/adr/0005-mixed-catalog-on-by-default.md`）。
fn active_catalog_plugin(cfg: &Config) -> CatalogPlugin {
    let mut resolved = cmrt_server_config::resolve_patch_catalog(
        cfg.plugin_id.as_deref(),
        &cfg.plugin_path,
        cfg.patches_dirs.as_deref(),
    );
    // The legacy/default profile deliberately surfaces a misspelled ordinary directory as a
    // collection error. Adapter-resolved catalogs keep their stricter loadable-source result.
    if resolved.resolved_patches.is_none() {
        resolved.dirs = configured_patch_dirs(cfg);
        resolved.configured_missing.clear();
    }
    if let Some(source_error) = resolved.source_error.take() {
        resolved
            .notices
            .push(format!("ロード可能な音色 source がない: {source_error}"));
    }
    let dirs = resolved.dirs;
    CatalogPlugin {
        name: cfg
            .active_plugin
            .clone()
            .unwrap_or_else(|| crate::plugin_file_stem(&cfg.plugin_path).to_string()),
        plugin_path: cfg.plugin_path.clone(),
        plugin_id: cfg.plugin_id.clone(),
        base: shared_patch_root_dir(&dirs),
        dirs,
        resolved_patches: resolved.resolved_patches,
        source_notices: resolved.notices,
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
fn installed_plugin_profiles(cfg: &Config) -> Vec<InstalledProfile> {
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
            let resolved = cmrt_server_config::resolve_patch_catalog(
                profile.plugin_id.as_deref(),
                &profile.plugin_path,
                profile.patches_dirs.as_deref(),
            );
            InstalledProfile {
                name,
                profile: PluginProfile {
                    patches_dirs: Some(resolved.dirs),
                    ..profile
                },
                missing_dirs: resolved.configured_missing,
                resolved_patches: resolved.resolved_patches,
                source_notices: resolved.notices,
                source_error: resolved.source_error,
            }
        })
        .collect()
}

/// [`installed_plugin_profiles`] が返す 1 つぶん。
///
/// `missing_dirs` を捨てずに持ち回るのは、音色置き場が 1 つも残らなかったときに
/// 「書いていない」のか「書いたが実在しない」のかを言い分けるため。
/// 捨ててしまうと、綴りを間違えた dir が「未設定です」と案内されて直しようがない。
struct InstalledProfile {
    name: String,
    /// `patches_dirs` は**実在する dir だけ**に絞り込んである。
    profile: PluginProfile,
    /// 書かれていたが実在しなかった dir。
    missing_dirs: Vec<String>,
    /// Adapter が解決済みの file paths。通常 scanner を使う plugin は `None`。
    resolved_patches: Option<Vec<PathBuf>>,
    /// Partial source failures and exclusions.
    source_notices: Vec<String>,
    /// No loadable source could be resolved.
    source_error: Option<String>,
}

/// 同じプラグインを 2 度カタログへ載せないための同定。
///
/// 既定プラグインはトップレベルへ焼き込み済みで、由来のプロファイル名が残っていない。
/// 名前では突き合わせられないので、プラグインそのものの同一性で見る。
///
/// `plugin_id` は `active_plugin` を使わない旧 config には書かれていないので、
/// **両方に書かれているときだけ** それで判定し、無ければ `plugin_path` のファイル名で
/// 見る。これにより、既定プラグインの
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
