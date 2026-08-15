use std::collections::{HashMap, VecDeque};

use super::AUDIO_CACHE_MAX_ENTRIES;

// patch 表示パス / 文字列リストのフィルタ（filter_patches_by_display_path / filter_items）は
// notepad と DAW の双方から使うため `cmrt-tui-core` へ切り出した。
// 従来の `crate::filter_*` パスは `tui.rs` の再エクスポートで維持する。

/// キャッシュからサンプルを取得する。
/// キャッシュ参照がない場合は `None` を返す。
pub(super) fn resolve_cached_samples(
    cache: Option<&HashMap<String, Vec<f32>>>,
    mml: &str,
) -> Option<Vec<f32>> {
    cache.and_then(|cache| cache.get(mml).cloned())
}

fn compact_cache_order(cache: &HashMap<String, Vec<f32>>, order: &mut VecDeque<String>) {
    let mut normalized = VecDeque::with_capacity(cache.len());
    while let Some(key) = order.pop_front() {
        if cache.contains_key(&key) && !normalized.iter().any(|existing| existing == &key) {
            normalized.push_back(key);
        }
    }
    for key in cache.keys() {
        if !normalized.iter().any(|existing| existing == key) {
            normalized.push_back(key.clone());
        }
    }
    *order = normalized;
}

pub(super) fn mark_cache_entry_recent(
    cache: &HashMap<String, Vec<f32>>,
    order: &mut VecDeque<String>,
    mml: &str,
) {
    compact_cache_order(cache, order);
    if !cache.contains_key(mml) {
        return;
    }
    order.retain(|key| key != mml);
    order.push_back(mml.to_string());
}

/// キャッシュにサンプルを挿入する。上限に達した場合は古い1件を退避してから挿入する。
/// `random_patch` が true の場合は何もしない。
///
/// 呼び出し元は `audio_cache` と `audio_cache_order` のロックを保持した状態で、
/// `&mut HashMap` と `&mut VecDeque` を渡すこと。
/// この関数自体は非同期に呼び出されないため、退避対象の決定と insert は事実上アトミックである。
pub(super) fn try_insert_cache(
    cache: &mut HashMap<String, Vec<f32>>,
    order: &mut VecDeque<String>,
    mml: String,
    samples: Vec<f32>,
    random_patch: bool,
) {
    if random_patch {
        return;
    }

    compact_cache_order(cache, order);

    if cache.contains_key(&mml) {
        cache.insert(mml.clone(), samples);
        mark_cache_entry_recent(cache, order, &mml);
        return;
    }

    if cache.len() >= AUDIO_CACHE_MAX_ENTRIES {
        while let Some(evicted) = order.pop_front() {
            if cache.remove(&evicted).is_some() {
                break;
            }
        }
    }

    cache.insert(mml.clone(), samples);
    order.push_back(mml);
}
