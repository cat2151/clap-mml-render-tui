use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn shared_patch_root_dir_returns_single_dir_as_is() {
    let dirs = vec!["/tmp/patches_factory".to_string()];

    let base = shared_patch_root_dir(&dirs);

    assert_eq!(base.as_deref(), Some("/tmp/patches_factory"));
}

#[test]
fn shared_patch_root_dir_returns_common_parent_for_multiple_dirs() {
    let dirs = vec![
        "/tmp/surge-data/patches_factory".to_string(),
        "/tmp/surge-data/patches_3rdparty".to_string(),
    ];

    let base = shared_patch_root_dir(&dirs);

    assert_eq!(base.as_deref(), Some("/tmp/surge-data"));
}

#[test]
fn shared_patch_root_dir_returns_none_when_only_empty_root_matches() {
    let dirs = vec![
        "patches_factory".to_string(),
        "patches_3rdparty".to_string(),
    ];

    let base = shared_patch_root_dir(&dirs);

    assert_eq!(base, None);
}

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
        realtime_audio_backend: cmrt_runtime::RealtimeAudioBackend::InProcess,
        realtime_play_server_port: cmrt_runtime::DEFAULT_REALTIME_PLAY_SERVER_PORT,
        realtime_play_server_command: String::new(),
        realtime_play_server_prewarm: false,
        autoplay_on_startup: true,
        voicing_shared_source: String::new(),
        voicing_override_source: String::new(),
        chord_progression_source: String::new(),
        chord_patch_categories: Vec::new(),
        bass_patch_categories: Vec::new(),
        arpeggio_patch_categories: Vec::new(),
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
        realtime_audio_backend: cmrt_runtime::RealtimeAudioBackend::InProcess,
        realtime_play_server_port: cmrt_runtime::DEFAULT_REALTIME_PLAY_SERVER_PORT,
        realtime_play_server_command: String::new(),
        realtime_play_server_prewarm: false,
        autoplay_on_startup: true,
        voicing_shared_source: String::new(),
        voicing_override_source: String::new(),
        chord_progression_source: String::new(),
        chord_patch_categories: Vec::new(),
        bass_patch_categories: Vec::new(),
        arpeggio_patch_categories: Vec::new(),
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
fn filter_patches_matches_every_term_case_insensitively() {
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
        filter_patches(&all, "WARM pad"),
        vec!["patches_factory/Pads/Warm Pad.fxp".to_string()]
    );
    assert_eq!(filter_patches(&all, "   ").len(), 2);
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
