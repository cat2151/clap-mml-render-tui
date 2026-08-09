//! 設定された patch ディレクトリからの patch 一覧収集と、一覧の絞り込み。
//!
//! Surge XT のディレクトリ構造・カテゴリ体系の知識は [`cmrt_surge_patches`] に閉じてある。
//! ここは `Config`（＝設定された patch dir）と、別 repo の `.fxp` 走査
//! （`cmrt_core::collect_patches`）を繋ぐ層に徹する。

use anyhow::Result;
use cmrt_runtime::{configured_patch_dirs, shared_patch_root_dir, Config};
use cmrt_surge_patches::{sort_patch_pairs, PatchSortOrder};

pub fn has_configured_patch_dirs(cfg: &Config) -> bool {
    !configured_patch_dirs(cfg).is_empty()
}

pub fn collect_patch_pairs(cfg: &Config) -> Result<Vec<(String, String)>> {
    let dirs = configured_patch_dirs(cfg);
    let Some(base_dir) = shared_patch_root_dir(&dirs) else {
        return collect_patch_pairs_with_optional_base(&dirs, None);
    };
    collect_patch_pairs_with_optional_base(&dirs, Some(base_dir.as_str()))
}

fn collect_patch_pairs_with_optional_base(
    dirs: &[String],
    base_dir: Option<&str>,
) -> Result<Vec<(String, String)>> {
    let mut pairs = Vec::new();
    for dir in dirs {
        let paths = cmrt_core::collect_patches(dir)?;
        pairs.extend(paths.into_iter().map(|path| {
            let display = match base_dir {
                Some(base_dir) => cmrt_core::to_relative(base_dir, &path),
                None => path.to_string_lossy().into_owned(),
            };
            let lower = display.to_lowercase();
            (display, lower)
        }));
    }
    sort_patch_pairs(&mut pairs, PatchSortOrder::Path);
    Ok(pairs)
}

/// クエリ文字列（空白区切りでAND条件）でパッチリストをフィルタする。
/// `all` は (表示名, 小文字化済み表示名) のペアであること（起動時に一度だけ計算）。
pub fn filter_patches(all: &[(String, String)], query: &str) -> Vec<String> {
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
