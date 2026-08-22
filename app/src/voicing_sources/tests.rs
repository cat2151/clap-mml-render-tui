use std::{fs, path::PathBuf};

use super::*;

use cmrt_tui_core::patch_plugins::{CatalogPlugin, PatchPlugins};

fn catalog(plugins: &[CatalogPlugin]) -> PatchPlugins {
    PatchPlugins::from_catalog(plugins.to_vec())
}

fn surge_plugin() -> CatalogPlugin {
    catalog_plugin(
        cmrt_runtime::SURGE_XT_PLUGIN_ID,
        cmrt_runtime::default_plugin_path(),
    )
}

fn vaporizer2_plugin() -> CatalogPlugin {
    catalog_plugin(
        cmrt_runtime::VAPORIZER2_PLUGIN_ID,
        r"C:\Program Files\Common Files\CLAP\VASTvaporizer2.clap",
    )
}

fn dexed_plugin() -> CatalogPlugin {
    catalog_plugin(
        cmrt_runtime::DEXED_PLUGIN_ID,
        cmrt_runtime::default_dexed_plugin_path(),
    )
}

fn catalog_plugin(plugin_id: &str, plugin_path: &str) -> CatalogPlugin {
    CatalogPlugin {
        name: String::new(),
        plugin_path: plugin_path.to_string(),
        plugin_id: Some(plugin_id.to_string()),
        base: None,
        dirs: Vec::new(),
        patch_roles: cmrt_runtime::PatchRoles::default(),
    }
}

fn unique_temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cmrt-voicing-source-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn json_entry(patch: &str, voicing: &str) -> String {
    format!(r#"{{"entries":{{"{patch}":"{voicing}"}}}}"#)
}

fn spawn_refresh_for_test(sources: SourceSet) -> VoicingSourceRefresh {
    let refresh = VoicingSourceRefresh {
        sources: Some(sources),
        completion: Arc::new((Mutex::new(false), Condvar::new())),
        io_lock: Arc::new(Mutex::new(())),
    };
    refresh.spawn_worker();
    refresh
}

#[test]
fn voicing_layers_resolve_override_then_user_then_shared() {
    let mut shared = VoicingCache::default();
    let mut user = VoicingCache::default();
    let mut override_ = VoicingCache::default();
    shared.insert("Leads/Shared.fxp", PatchVoicing::Poly);
    shared.insert("Leads/User.fxp", PatchVoicing::Poly);
    shared.insert("Leads/Override.fxp", PatchVoicing::Poly);
    user.insert("Leads/User.fxp", PatchVoicing::Mono);
    user.insert("Leads/Override.fxp", PatchVoicing::Poly);
    override_.insert("Leads/Override.fxp", PatchVoicing::Mono);
    let layers = VoicingLayers { shared, override_ };

    assert_eq!(
        layers.resolve(&user, "Leads/Override.fxp"),
        Some(PatchVoicing::Mono)
    );
    assert_eq!(
        layers.resolve(&user, "Leads/User.fxp"),
        Some(PatchVoicing::Mono)
    );
    assert_eq!(
        layers.resolve(&user, "Leads/Shared.fxp"),
        Some(PatchVoicing::Poly)
    );
}

#[test]
fn shared_json_normalizes_case_and_separators() {
    let cache = VoicingCache::from_shared_json(
        r#"{"entries":{"Patches_3rdParty\\Slowboat/Winds/Clarinet.fxp":"mono"}}"#,
    )
    .unwrap();

    assert_eq!(
        cache.get("patches_3rdparty/slowboat/winds/clarinet.fxp"),
        Some(PatchVoicing::Mono)
    );
}

#[test]
fn invalid_voicing_json_is_rejected_before_it_reaches_the_cache() {
    assert!(validate_voicing_json(json_entry("Leads/A.fxp", "mono").as_bytes()).is_ok());
    assert!(validate_voicing_json(b"not json").is_err());
}

#[test]
fn first_keyboard_entry_waits_for_missing_local_source() {
    let temp = unique_temp_dir("first-load");
    fs::create_dir_all(&temp).unwrap();
    fs::write(
        temp.join("shared-source.json"),
        json_entry("Leads/First.fxp", "mono"),
    )
    .unwrap();
    let sources = SourceSet::new(&temp, "shared-source.json".to_string(), String::new());
    let refresh = spawn_refresh_for_test(sources);

    let layers = refresh.load_for_keyboard();

    assert_eq!(
        layers.resolve(&VoicingCache::default(), "Leads/First.fxp"),
        Some(PatchVoicing::Mono)
    );
    fs::remove_dir_all(temp).ok();
}

#[test]
fn first_keyboard_entry_continues_when_source_is_unavailable() {
    let temp = unique_temp_dir("first-failure");
    let sources = SourceSet::new(&temp, "missing.json".to_string(), String::new());
    let refresh = spawn_refresh_for_test(sources);

    let layers = refresh.load_for_keyboard();

    assert_eq!(
        layers.resolve(&VoicingCache::default(), "Leads/Missing.fxp"),
        None
    );
    fs::remove_dir_all(temp).ok();
}

#[test]
fn persisted_sources_are_read_again_for_each_keyboard_entry() {
    let temp = unique_temp_dir("reload");
    let sources = SourceSet::new(&temp, String::new(), "override-source".to_string());
    fs::create_dir_all(sources.override_.data_path.parent().unwrap()).unwrap();
    fs::write(
        &sources.override_.data_path,
        json_entry("Winds/Reload.fxp", "poly"),
    )
    .unwrap();
    let first = load_persisted_cache(&sources.override_);
    fs::write(
        &sources.override_.data_path,
        json_entry("Winds/Reload.fxp", "mono"),
    )
    .unwrap();
    let second = load_persisted_cache(&sources.override_);

    assert_eq!(first.get("Winds/Reload.fxp"), Some(PatchVoicing::Poly));
    assert_eq!(second.get("Winds/Reload.fxp"), Some(PatchVoicing::Mono));
    fs::remove_dir_all(temp).ok();
}

/// Surge 専用 JSON を取りに行くかは、既定プラグインではなく**カタログ全体**で決まる。
/// カタログに Surge の音色が 1 つでも載るなら、既定プラグインが Dexed でも JSON は要る。
#[test]
fn surge_only_sources_follow_the_whole_catalog() {
    let cfg = Config {
        plugin_id: Some(cmrt_runtime::DEXED_PLUGIN_ID.to_string()),
        plugin_path: cmrt_runtime::default_dexed_plugin_path().to_string(),
        ..Config::default()
    };

    assert!(
        SourceSet::from_catalog(&cfg, &catalog(&[surge_plugin()])).is_some(),
        "Surge XT では共有 voicing データを読む"
    );
    assert!(
        SourceSet::from_catalog(&cfg, &catalog(&[dexed_plugin(), surge_plugin()])).is_some(),
        "既定が Dexed でも、カタログに Surge が載るなら読む"
    );
    assert!(
        SourceSet::from_catalog(&cfg, &catalog(&[dexed_plugin()])).is_none(),
        "カタログに Surge が載らないなら、Surge 専用 JSON を取りに行かない"
    );
    assert!(
        SourceSet::from_catalog(&cfg, &catalog(&[dexed_plugin(), vaporizer2_plugin()])).is_none(),
        "Vaporizer2 の mono/poly は `.vvp` に書いてあるので、この JSON は要らない"
    );
}

/// 取りに行かない＝レイヤは常に空。判定は `VoicingPolicy` が受け持つ。
#[test]
fn layers_are_empty_without_sources() {
    assert_eq!(
        VoicingSourceRefresh::disabled().load_for_keyboard(),
        VoicingLayers::default()
    );
}
