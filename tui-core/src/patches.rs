//! 設定された patch ディレクトリからの patch 一覧収集と、一覧の絞り込み。
//!
//! Surge XT のディレクトリ構造・カテゴリ体系の知識は [`cmrt_patches`] に閉じてある。
//! ここは `Config`（＝設定された patch dir）と、別 repo の `.fxp` 走査
//! （`cmrt_core::collect_patches`）を繋ぐ層に徹する。

use anyhow::Result;
use cmrt_core::MergedPatches;
use cmrt_patches::{sort_patch_pairs, PatchSortOrder};
use cmrt_runtime::{catalog_plugins, configured_patch_dirs, CatalogPlugin, Config, PatchBase};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub fn has_configured_patch_dirs(cfg: &Config) -> bool {
    !configured_patch_dirs(cfg).is_empty()
}

/// 設定された patch ディレクトリを走査し、(表示パス, 小文字化済み表示パス) の一覧を作る。
///
/// 相対化の基点は [`CatalogPlugin`] ごとに別。プラグインを跨いだ共通の親で相対化すると
/// display 文字列（＝永続 ID）が変わってしまうため、**プラグインごとに相対化してから
/// 連結する**。並べ替えは連結後に 1 回だけ行うので、プラグインが増えても一覧は
/// 表示パス順のまま混ざる。
pub fn collect_patch_pairs(cfg: &Config) -> Result<Vec<(String, String)>> {
    collect_patch_pairs_from_catalog(&catalog_plugins(cfg))
}

/// 解決済みcatalogを再利用してpatch一覧を収集する。
/// catalog resolverを一度だけ実行したいcache構築経路向け。
pub fn collect_patch_pairs_from_catalog(
    plugins: &[CatalogPlugin],
) -> Result<Vec<(String, String)>> {
    Ok(collect_patch_listing_from_catalog(plugins)?.pairs)
}

/// [`collect_patch_pairs_from_catalog`] の一覧と、同じ音をまとめた patch の情報。
pub struct PatchListing {
    pub pairs: Vec<(String, String)>,
    /// display → その patch へまとめた件数と名前。まとめていない patch は載らない。
    pub merged: HashMap<String, MergedPatches>,
}

/// 同じ音を鳴らす patch を play server 側でまとめた一覧を集める。
pub fn collect_patch_listing_from_catalog(plugins: &[CatalogPlugin]) -> Result<PatchListing> {
    let mut listing = PatchListing {
        pairs: Vec::new(),
        merged: HashMap::new(),
    };
    let mut seen = HashSet::new();
    for plugin in plugins {
        extend_with_plugin(&mut listing, &mut seen, plugin)?;
    }
    sort_patch_pairs(&mut listing.pairs, PatchSortOrder::Path);
    Ok(listing)
}

fn extend_with_plugin(
    listing: &mut PatchListing,
    seen: &mut HashSet<String>,
    plugin: &CatalogPlugin,
) -> Result<()> {
    if let Some(paths) = &plugin.resolved_patches {
        extend_with_paths(
            listing,
            seen,
            plugin,
            paths.iter().map(|path| (path.clone(), None)),
        );
    } else {
        for dir in &plugin.dirs {
            let patches = cmrt_core::collect_patch_listing(dir)?;
            extend_with_paths(
                listing,
                seen,
                plugin,
                patches.into_iter().map(|patch| (patch.path, patch.merged)),
            );
        }
    }
    Ok(())
}

fn extend_with_paths(
    listing: &mut PatchListing,
    seen: &mut HashSet<String>,
    plugin: &CatalogPlugin,
    patches: impl IntoIterator<Item = (PathBuf, Option<MergedPatches>)>,
) {
    for (path, merged) in patches {
        let canonical = cmrt_core::lexical_absolute(&path).unwrap_or_else(|_| path.clone());
        if !seen.insert(canonical_key(&canonical)) {
            continue;
        }
        let display = match &plugin.base {
            PatchBase::Shared(base) => relative_display(base, &path),
            PatchBase::PerRoot(_) => plugin.base.display(&canonical),
            PatchBase::None => path.to_string_lossy().into_owned(),
        };
        if let Some(merged) = merged {
            listing.merged.insert(display.clone(), merged);
        }
        let lower = display.to_lowercase();
        listing.pairs.push((display, lower));
    }
}

fn relative_display(base: &str, path: &Path) -> String {
    match (
        cmrt_core::lexical_absolute(Path::new(base)),
        cmrt_core::lexical_absolute(path),
    ) {
        (Ok(base), Ok(path)) => cmrt_core::to_relative(&base.to_string_lossy(), &path),
        _ => cmrt_core::to_relative(base, path),
    }
}

fn canonical_key(path: &Path) -> String {
    let key = path.to_string_lossy().into_owned();
    if cfg!(windows) {
        key.to_lowercase()
    } else {
        key
    }
}

/// クエリ文字列（空白区切りでAND条件）で patch の表示パス全文をフィルタする。
///
/// category、vendor、filename のすべてが検索対象になる。filename stem だけを探す
/// grid sequencer の patch name 検索とは検索範囲が異なるため、共通化しない。
/// `all` は (表示パス, 小文字化済み表示パス) のペアであること（起動時に一度だけ計算）。
pub fn filter_patches_by_display_path(all: &[(String, String)], query: &str) -> Vec<String> {
    let terms: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();
    if terms.is_empty() {
        return all.iter().map(|(orig, _)| orig.clone()).collect();
    }
    all.iter()
        .filter(|(_, lower)| terms.iter().all(|t| lower.contains(t.as_str())))
        .map(|(orig, _)| orig.clone())
        .collect()
}

/// クエリ文字列（空白区切りでAND条件）で文字列リストをフィルタする。
pub fn filter_items(items: &[String], query: &str) -> Vec<String> {
    let terms: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();
    if terms.is_empty() {
        return items.to_vec();
    }
    items
        .iter()
        .filter(|item| {
            let lower = item.to_lowercase();
            terms.iter().all(|term| lower.contains(term.as_str()))
        })
        .cloned()
        .collect()
}

/// 表示パスの末尾要素から落とす拡張子。
///
/// **末尾のドット以降を無条件に落としてはいけない。** Dexed の音色名には
/// `05 T.BL-EXPA` や `14 P.ICE 25.1` のようにドットを含むものが実在し、
/// 無条件に落とすと名前が欠ける。既知の拡張子だけを対象にする。
const PATCH_FILE_EXTENSIONS: [&str; 8] = [
    "fxp",
    "sfz",
    "ariax",
    "vvp",
    "syx",
    "floe-preset",
    "sxsnp",
    "h2p",
];

/// 表示パスの末尾要素から、既知の拡張子だけを落とした音色名。
///
/// - `patches_3rdparty/Dan Maurer/Winds/Reed To Pipe Morph.fxp` → `Reed To Pipe Morph`
/// - `SynprezFM/SynprezFM_22.syx/05 SampleSqr2` → `05 SampleSqr2`
pub fn patch_stem(display: &str) -> &str {
    let last = display.rsplit(['/', '\\']).next().unwrap_or(display).trim();
    match last.rsplit_once('.') {
        Some((stem, extension))
            if !stem.is_empty()
                && PATCH_FILE_EXTENSIONS
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(extension)) =>
        {
            stem
        }
        _ => last,
    }
}

#[cfg(test)]
mod tests;
