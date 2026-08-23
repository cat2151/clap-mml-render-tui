//! 設定された patch ディレクトリからの patch 一覧収集と、一覧の絞り込み。
//!
//! Surge XT のディレクトリ構造・カテゴリ体系の知識は [`cmrt_patches`] に閉じてある。
//! ここは `Config`（＝設定された patch dir）と、別 repo の `.fxp` 走査
//! （`cmrt_core::collect_patches`）を繋ぐ層に徹する。

use anyhow::Result;
use cmrt_patches::{sort_patch_pairs, PatchSortOrder};
use cmrt_runtime::{catalog_plugins, configured_patch_dirs, CatalogPlugin, Config};
use std::collections::HashSet;
use std::path::Path;

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
    let mut pairs = Vec::new();
    let mut seen = HashSet::new();
    for plugin in plugins {
        extend_with_plugin(&mut pairs, &mut seen, plugin)?;
    }
    sort_patch_pairs(&mut pairs, PatchSortOrder::Path);
    Ok(pairs)
}

fn extend_with_plugin(
    pairs: &mut Vec<(String, String)>,
    seen: &mut HashSet<String>,
    plugin: &CatalogPlugin,
) -> Result<()> {
    if let Some(paths) = &plugin.resolved_patches {
        extend_with_paths(pairs, seen, plugin, paths.iter().cloned());
    } else {
        for dir in &plugin.dirs {
            let paths = cmrt_core::collect_patches(dir)?;
            extend_with_paths(pairs, seen, plugin, paths);
        }
    }
    Ok(())
}

fn extend_with_paths(
    pairs: &mut Vec<(String, String)>,
    seen: &mut HashSet<String>,
    plugin: &CatalogPlugin,
    paths: impl IntoIterator<Item = std::path::PathBuf>,
) {
    pairs.extend(paths.into_iter().filter_map(|path| {
        let canonical = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if !seen.insert(canonical_key(&canonical)) {
            return None;
        }
        let display = match plugin.base.as_deref() {
            Some(base) => relative_display(base, &path),
            None => path.to_string_lossy().into_owned(),
        };
        let lower = display.to_lowercase();
        Some((display, lower))
    }));
}

fn relative_display(base: &str, path: &Path) -> String {
    match (std::fs::canonicalize(base), std::fs::canonicalize(path)) {
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

#[cfg(test)]
mod tests;
