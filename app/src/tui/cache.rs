use std::collections::HashMap;

use super::AUDIO_CACHE_MAX_ENTRIES;

/// クエリ文字列（空白区切りでAND条件）でパッチリストをフィルタする。
/// `all` は (表示名, 小文字化済み表示名) のペアであること（起動時に一度だけ計算）。
pub(super) fn filter_patches(all: &[(String, String)], query: &str) -> Vec<String> {
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
pub(super) fn filter_items(items: &[String], query: &str) -> Vec<String> {
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

/// キャッシュからサンプルを取得する。
/// キャッシュ参照がない場合は `None` を返す。
pub(super) fn resolve_cached_samples(
    cache: Option<&HashMap<String, Vec<f32>>>,
    mml: &str,
) -> Option<Vec<f32>> {
    cache.and_then(|cache| cache.get(mml).cloned())
}

/// キャッシュにサンプルを挿入する。上限に達した場合はキャッシュ全体をクリアしてから挿入する。
/// `random_patch` が true の場合は何もしない。
///
/// 呼び出し元は `audio_cache` のロックを保持した状態で `&mut HashMap` を渡すこと。
/// この関数自体は非同期に呼び出されないため、len 確認と insert は事実上アトミックである。
pub(super) fn try_insert_cache(
    cache: &mut HashMap<String, Vec<f32>>,
    mml: String,
    samples: Vec<f32>,
    random_patch: bool,
) {
    if random_patch {
        return;
    }
    if cache.len() >= AUDIO_CACHE_MAX_ENTRIES && !cache.contains_key(&mml) {
        cache.clear();
    }
    cache.insert(mml, samples);
}
