use super::*;

use std::collections::VecDeque;

#[test]
fn filter_patches_empty_query_returns_all() {
    let all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    let result = filter_patches(&all, "");
    assert_eq!(result, vec!["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
}

#[test]
fn filter_patches_single_term_matches_substring() {
    let all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    let result = filter_patches(&all, "pad");
    assert_eq!(result, vec!["Pads/Pad 1.fxp"]);
}

#[test]
fn filter_patches_case_insensitive() {
    let all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    let result = filter_patches(&all, "PAD");
    assert_eq!(result, vec!["Pads/Pad 1.fxp"]);
}

#[test]
fn filter_patches_multiple_terms_act_as_and() {
    let all = make_patches(&["Pads/Soft Pad.fxp", "Pads/Hard Pad.fxp", "Leads/Lead 1.fxp"]);
    let result = filter_patches(&all, "pad soft");
    assert_eq!(result, vec!["Pads/Soft Pad.fxp"]);
}

#[test]
fn filter_patches_no_match_returns_empty() {
    let all = make_patches(&["Pads/Pad 1.fxp"]);
    let result = filter_patches(&all, "xyznomatch");
    assert!(result.is_empty());
}

#[test]
fn filter_patches_whitespace_only_query_returns_all() {
    let all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    // split_whitespace で空のイテレータになり、全件返す
    let result = filter_patches(&all, "   ");
    assert_eq!(result, vec!["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
}

#[test]
fn filter_patches_empty_list_returns_empty() {
    let all: Vec<(String, String)> = vec![];
    let result = filter_patches(&all, "pad");
    assert!(result.is_empty());
}

#[test]
fn filter_items_multiple_terms_act_as_and() {
    let items = vec![
        "Pads/Soft Pad.fxp".to_string(),
        "Pads/Hard Pad.fxp".to_string(),
        "Leads/Lead 1.fxp".to_string(),
    ];
    let result = filter_items(&items, "pad soft");
    assert_eq!(result, vec!["Pads/Soft Pad.fxp"]);
}

#[test]
fn filter_items_whitespace_only_query_returns_all() {
    let items = vec!["alpha beta".to_string(), "gamma".to_string()];
    let result = filter_items(&items, "   ");
    assert_eq!(result, items);
}

// --- audio cache helper tests ---

#[test]
fn resolve_cached_samples_returns_samples_on_cache_hit() {
    let mut cache = HashMap::new();
    cache.insert("cde".to_string(), vec![0.5f32, 0.6]);
    let result = resolve_cached_samples(Some(&cache), "cde");
    assert_eq!(result, Some(vec![0.5f32, 0.6]));
}

#[test]
fn mark_cache_entry_recent_moves_hit_key_to_back() {
    let mut cache = HashMap::new();
    cache.insert("old".to_string(), vec![]);
    cache.insert("hit".to_string(), vec![0.5f32]);
    cache.insert("new".to_string(), vec![]);
    let mut order = VecDeque::from(["old".to_string(), "hit".to_string(), "new".to_string()]);

    mark_cache_entry_recent(&cache, &mut order, "hit");

    assert_eq!(
        order,
        VecDeque::from(["old".to_string(), "new".to_string(), "hit".to_string()])
    );
}

#[test]
fn resolve_cached_samples_returns_none_on_cache_miss() {
    let cache: HashMap<String, Vec<f32>> = HashMap::new();
    let result = resolve_cached_samples(Some(&cache), "cde");
    assert!(result.is_none());
}

#[test]
fn resolve_cached_samples_returns_none_without_cache_reference() {
    let mut cache = HashMap::new();
    cache.insert("cde".to_string(), vec![0.0f32, 1.0]);
    let result = resolve_cached_samples(None, "cde");
    assert!(result.is_none());
}

#[test]
fn try_insert_cache_does_nothing_when_random_patch_true() {
    let mut cache = HashMap::new();
    let mut order = VecDeque::new();
    try_insert_cache(
        &mut cache,
        &mut order,
        "cde".to_string(),
        vec![1.0f32],
        true,
    );
    assert!(cache.is_empty());
    assert!(order.is_empty());
}

#[test]
fn try_insert_cache_inserts_when_random_patch_false() {
    let mut cache = HashMap::new();
    let mut order = VecDeque::new();
    try_insert_cache(
        &mut cache,
        &mut order,
        "cde".to_string(),
        vec![1.0f32],
        false,
    );
    assert!(cache.contains_key("cde"));
    assert_eq!(order, VecDeque::from(["cde".to_string()]));
}

#[test]
fn try_insert_cache_evicts_single_oldest_entry_when_full() {
    let mut cache = HashMap::new();
    let mut order = VecDeque::new();
    // AUDIO_CACHE_MAX_ENTRIES まで埋める
    for i in 0..AUDIO_CACHE_MAX_ENTRIES {
        let key = format!("mml_{}", i);
        cache.insert(key.clone(), vec![]);
        order.push_back(key);
    }
    assert_eq!(cache.len(), AUDIO_CACHE_MAX_ENTRIES);

    try_insert_cache(
        &mut cache,
        &mut order,
        "new_mml".to_string(),
        vec![0.1f32],
        false,
    );

    assert_eq!(cache.len(), AUDIO_CACHE_MAX_ENTRIES);
    assert!(!cache.contains_key("mml_0"));
    assert!(cache.contains_key("new_mml"));
    assert_eq!(order.len(), AUDIO_CACHE_MAX_ENTRIES);
}

#[test]
fn try_insert_cache_updates_existing_key_when_full() {
    let mut cache = HashMap::new();
    let mut order = VecDeque::new();
    // "cde" を含めてちょうど AUDIO_CACHE_MAX_ENTRIES 件になるよう埋める
    for i in 0..(AUDIO_CACHE_MAX_ENTRIES - 1) {
        let key = format!("mml_{}", i);
        cache.insert(key.clone(), vec![]);
        order.push_back(key);
    }
    cache.insert("cde".to_string(), vec![]);
    order.push_back("cde".to_string());
    assert_eq!(cache.len(), AUDIO_CACHE_MAX_ENTRIES);

    try_insert_cache(
        &mut cache,
        &mut order,
        "cde".to_string(),
        vec![0.9f32],
        false,
    );
    assert_eq!(cache.len(), AUDIO_CACHE_MAX_ENTRIES);
    assert_eq!(cache["cde"], vec![0.9f32]);
    assert_eq!(order.back(), Some(&"cde".to_string()));
}
