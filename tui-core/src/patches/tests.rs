use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn collect_patch_pairs_combines_factory_and_thirdparty_using_common_base() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("cmrt_collect_patch_pairs_{suffix}"));
    let factory = root.join("patches_factory");
    let thirdparty = root.join("patches_3rdparty");
    std::fs::create_dir_all(factory.join("Pads")).unwrap();
    std::fs::create_dir_all(thirdparty.join("Leads")).unwrap();
    std::fs::write(factory.join("Pads").join("Factory Pad.fxp"), b"dummy").unwrap();
    std::fs::write(thirdparty.join("Leads").join("Third Lead.fxp"), b"dummy").unwrap();

    let cfg = Config {
        plugin_path: String::new(),
        input_midi: String::new(),
        output_midi: String::new(),
        output_wav: String::new(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patches_dirs: Some(vec![
            factory.to_string_lossy().into_owned(),
            thirdparty.to_string_lossy().into_owned(),
        ]),
        loop_dirs: Vec::new(),
        loop_categories: cmrt_runtime::default_loop_categories(),
        offline_render_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_WORKERS,
        offline_render_server_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
        offline_render_backend: cmrt_runtime::OfflineRenderBackend::InProcess,
        offline_render_server_port: cmrt_runtime::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
        offline_render_server_command: String::new(),
        realtime_audio_backend: cmrt_runtime::RealtimeAudioBackend::CachePlayer,
        realtime_play_server_port: cmrt_runtime::DEFAULT_REALTIME_PLAY_SERVER_PORT,
        realtime_play_server_command: String::new(),
        realtime_play_server_prewarm: false,
        autoplay_on_startup: true,
        voicing_shared_source: String::new(),
        voicing_override_source: String::new(),
        chord_progression_source: String::new(),
        ..Default::default()
    };

    let pairs = collect_patch_pairs(&cfg).unwrap();

    assert!(pairs.contains(&(
        "patches_factory/Pads/Factory Pad.fxp".to_string(),
        "patches_factory/pads/factory pad.fxp".to_string()
    )));
    assert!(pairs.contains(&(
        "patches_3rdparty/Leads/Third Lead.fxp".to_string(),
        "patches_3rdparty/leads/third lead.fxp".to_string()
    )));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn collect_patch_pairs_sorts_display_names_naturally() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("cmrt_collect_patch_pairs_natural_{suffix}"));
    let factory = root.join("patches_factory");
    let pads = factory.join("Pads");
    std::fs::create_dir_all(&pads).unwrap();
    std::fs::write(pads.join("Pad 11.fxp"), b"dummy").unwrap();
    std::fs::write(pads.join("Pad 2.fxp"), b"dummy").unwrap();
    std::fs::write(pads.join("Pad 1.fxp"), b"dummy").unwrap();

    let cfg = Config {
        plugin_path: String::new(),
        input_midi: String::new(),
        output_midi: String::new(),
        output_wav: String::new(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patches_dirs: Some(vec![factory.to_string_lossy().into_owned()]),
        loop_dirs: Vec::new(),
        loop_categories: cmrt_runtime::default_loop_categories(),
        offline_render_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_WORKERS,
        offline_render_server_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
        offline_render_backend: cmrt_runtime::OfflineRenderBackend::InProcess,
        offline_render_server_port: cmrt_runtime::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
        offline_render_server_command: String::new(),
        realtime_audio_backend: cmrt_runtime::RealtimeAudioBackend::CachePlayer,
        realtime_play_server_port: cmrt_runtime::DEFAULT_REALTIME_PLAY_SERVER_PORT,
        realtime_play_server_command: String::new(),
        realtime_play_server_prewarm: false,
        autoplay_on_startup: true,
        voicing_shared_source: String::new(),
        voicing_override_source: String::new(),
        chord_progression_source: String::new(),
        ..Default::default()
    };

    let pairs = collect_patch_pairs(&cfg).unwrap();

    assert_eq!(
        pairs
            .into_iter()
            .map(|(display, _)| display)
            .collect::<Vec<_>>(),
        vec![
            "Pads/Pad 1.fxp".to_string(),
            "Pads/Pad 2.fxp".to_string(),
            "Pads/Pad 11.fxp".to_string(),
        ]
    );

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn filter_patches_by_display_path_matches_every_term_case_insensitively() {
    let all = vec![
        (
            "patches_factory/Pads/Warm Pad.fxp".to_string(),
            "patches_factory/pads/warm pad.fxp".to_string(),
        ),
        (
            "patches_factory/Leads/Warm Lead.fxp".to_string(),
            "patches_factory/leads/warm lead.fxp".to_string(),
        ),
    ];

    assert_eq!(
        filter_patches_by_display_path(&all, "WARM pad"),
        vec!["patches_factory/Pads/Warm Pad.fxp".to_string()]
    );
    assert_eq!(filter_patches_by_display_path(&all, "   ").len(), 2);
}

#[test]
fn display_path_filter_searches_category_vendor_and_patch_name() {
    let all = vec![
        (
            "patches_factory/Instrument/Soft Strum.fxp".to_string(),
            "patches_factory/instrument/soft strum.fxp".to_string(),
        ),
        (
            "patches_3rdparty/Acme/Guitars/Plain Voice.fxp".to_string(),
            "patches_3rdparty/acme/guitars/plain voice.fxp".to_string(),
        ),
    ];

    assert_eq!(
        filter_patches_by_display_path(&all, "instrument"),
        vec!["patches_factory/Instrument/Soft Strum.fxp".to_string()]
    );
    assert_eq!(
        filter_patches_by_display_path(&all, "acme"),
        vec!["patches_3rdparty/Acme/Guitars/Plain Voice.fxp".to_string()]
    );
    assert_eq!(
        filter_patches_by_display_path(&all, "strum"),
        vec!["patches_factory/Instrument/Soft Strum.fxp".to_string()]
    );
}

#[test]
fn filter_items_matches_every_term_case_insensitively() {
    let items = vec!["Warm Pad".to_string(), "Warm Lead".to_string()];

    assert_eq!(
        filter_items(&items, "warm LEAD"),
        vec!["Warm Lead".to_string()]
    );
    assert_eq!(filter_items(&items, ""), items);
}

/// 基点の違うプラグインを連結しても、display は「そのプラグインだけを相対化したとき」と
/// ビット単位で同じになる。display 文字列は永続 ID なので、カタログにプラグインが
/// 増えても既存の音色の指し先が変わってはいけない（`docs/adr/0006-per-profile-relative-base.md`）。
///
/// 連結の規則は、開発機のインストール状況に左右されないよう `catalog_plugins` を
/// 通さずにここで直接確かめる。
#[test]
fn extending_with_two_plugins_keeps_each_display_relative_to_its_own_base() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("cmrt_patch_dir_groups_{suffix}"));
    let surge = root.join("surge-data").join("patches_factory");
    let cartridges = root.join("elsewhere").join("Cartridges");
    std::fs::create_dir_all(surge.join("Pads")).unwrap();
    std::fs::create_dir_all(&cartridges).unwrap();
    std::fs::write(surge.join("Pads").join("Factory Pad.fxp"), b"dummy").unwrap();
    std::fs::write(cartridges.join("Only Voice.fxp"), b"dummy").unwrap();

    let catalog = vec![
        catalog_plugin(
            root.join("surge-data").to_string_lossy().into_owned(),
            surge.to_string_lossy().into_owned(),
        ),
        catalog_plugin(
            cartridges.to_string_lossy().into_owned(),
            cartridges.to_string_lossy().into_owned(),
        ),
    ];

    let mut pairs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for plugin in &catalog {
        extend_with_plugin(&mut pairs, &mut seen, plugin).unwrap();
    }
    sort_patch_pairs(&mut pairs, PatchSortOrder::Path);

    assert_eq!(
        pairs
            .into_iter()
            .map(|(display, _)| display)
            .collect::<Vec<_>>(),
        vec![
            "Only Voice.fxp".to_string(),
            "patches_factory/Pads/Factory Pad.fxp".to_string(),
        ]
    );

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn adapter_resolved_paths_are_used_without_rescanning_vendor_files() {
    let root = std::env::temp_dir().join(format!("cmrt_sfz_overlap_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let bank = root.join("Bank");
    std::fs::create_dir_all(&bank).unwrap();
    std::fs::write(bank.join("Piano.sfz"), b"<region>").unwrap();
    std::fs::write(bank.join("Not Sforzando.fxp"), b"state").unwrap();
    let piano = std::fs::canonicalize(bank.join("Piano.sfz")).unwrap();
    let plugin = CatalogPlugin {
        name: "adapter fixture".to_string(),
        plugin_path: "adapter.clap".to_string(),
        plugin_id: Some("org.example.adapter".to_string()),
        base: Some(root.to_string_lossy().into_owned()),
        dirs: vec![
            root.to_string_lossy().into_owned(),
            bank.to_string_lossy().into_owned(),
        ],
        resolved_patches: Some(vec![piano]),
        source_notices: Vec::new(),
    };
    let mut pairs = Vec::new();
    let mut seen = std::collections::HashSet::new();

    extend_with_plugin(&mut pairs, &mut seen, &plugin).unwrap();

    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].0, "Bank/Piano.sfz");
    let _ = std::fs::remove_dir_all(root);
}

fn catalog_plugin(base: String, dir: String) -> CatalogPlugin {
    CatalogPlugin {
        name: "test".to_string(),
        plugin_path: String::new(),
        plugin_id: None,
        base: Some(base),
        dirs: vec![dir],
        resolved_patches: None,
        source_notices: Vec::new(),
    }
}
