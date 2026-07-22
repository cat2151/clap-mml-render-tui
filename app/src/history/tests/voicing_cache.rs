use super::*;

use crate::history::{load_voicing_cache, save_voicing_cache, VoicingCache};
use crate::realtime_play::PatchVoicing;

#[test]
fn save_and_load_voicing_cache_round_trips() {
    let tmp = std::env::temp_dir().join("cmrt_test_voicing_cache_roundtrip");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut cache = VoicingCache::default();
    assert!(cache.insert("patches_factory/Leads/Mono.fxp", PatchVoicing::Mono));
    assert!(cache.insert("patches_factory/Keys/Piano.fxp", PatchVoicing::Poly));
    save_voicing_cache(&cache).unwrap();

    let loaded = load_voicing_cache();
    assert_eq!(loaded, cache);
    assert_eq!(
        loaded.get("patches_factory/Leads/Mono.fxp"),
        Some(PatchVoicing::Mono)
    );
    assert_eq!(loaded.get("patches_factory/Pads/Warm.fxp"), None);

    assert_voicing_cache_file_path();
    std::fs::remove_dir_all(&tmp).ok();
}

fn assert_voicing_cache_file_path() {
    let path = super::voicing_cache_path().unwrap();
    assert_history_file_path(&path, "voicing_cache.json");
    assert!(path.exists(), "voicing_cache.json が保存されていない");
}

#[test]
fn load_voicing_cache_falls_back_to_default_for_broken_file() {
    let tmp = std::env::temp_dir().join("cmrt_test_voicing_cache_broken");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let path = super::voicing_cache_path().unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "not json").unwrap();

    assert_eq!(load_voicing_cache(), VoicingCache::default());
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn voicing_cache_key_ignores_separator_and_case() {
    let mut cache = VoicingCache::default();
    cache.insert("patches_factory/Leads/Mono.fxp", PatchVoicing::Mono);

    assert_eq!(
        cache.get("patches_factory\\Leads\\MONO.fxp"),
        Some(PatchVoicing::Mono)
    );
}

#[test]
fn voicing_cache_insert_reports_whether_it_changed() {
    let mut cache = VoicingCache::default();

    assert!(cache.insert("Leads/Mono.fxp", PatchVoicing::Mono));
    // 同じ結果の再登録は保存を要求しない
    assert!(!cache.insert("Leads/Mono.fxp", PatchVoicing::Mono));
    assert!(cache.insert("Leads/Mono.fxp", PatchVoicing::Poly));
}

#[test]
fn voicing_cache_does_not_store_unknown() {
    let mut cache = VoicingCache::default();

    assert!(!cache.insert("Leads/Mono.fxp", PatchVoicing::Unknown));
    assert_eq!(cache.get("Leads/Mono.fxp"), None);
}
