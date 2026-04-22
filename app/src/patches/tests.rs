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
        offline_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
        offline_render_server_workers: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
        offline_render_backend: crate::config::OfflineRenderBackend::InProcess,
        offline_render_server_port: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
        offline_render_server_command: String::new(),
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
        offline_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
        offline_render_server_workers: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
        offline_render_backend: crate::config::OfflineRenderBackend::InProcess,
        offline_render_server_port: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
        offline_render_server_command: String::new(),
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
fn compare_patch_names_natural_orders_numeric_suffixes() {
    let mut items = vec![
        "Pads/Pad 11.fxp".to_string(),
        "Pads/Pad 2.fxp".to_string(),
        "Pads/Pad 1.fxp".to_string(),
    ];
    items.sort_by(|left, right| compare_patch_names_natural(left, right));

    assert_eq!(
        items,
        vec![
            "Pads/Pad 1.fxp".to_string(),
            "Pads/Pad 2.fxp".to_string(),
            "Pads/Pad 11.fxp".to_string(),
        ]
    );
}

#[test]
fn compare_normalized_patch_names_natural_orders_numeric_suffixes() {
    let mut items = vec![
        "pads/pad 11.fxp".to_string(),
        "pads/pad 2.fxp".to_string(),
        "pads/pad 1.fxp".to_string(),
    ];
    items.sort_by(|left, right| compare_normalized_patch_names_natural(left, right));

    assert_eq!(
        items,
        vec![
            "pads/pad 1.fxp".to_string(),
            "pads/pad 2.fxp".to_string(),
            "pads/pad 11.fxp".to_string(),
        ]
    );
}

#[test]
fn resolve_display_patch_name_adds_factory_prefix_when_missing() {
    let pairs = vec![
        (
            "patches_factory/Pads/Factory Pad.fxp".to_string(),
            "patches_factory/pads/factory pad.fxp".to_string(),
        ),
        (
            "patches_3rdparty/Leads/Third Lead.fxp".to_string(),
            "patches_3rdparty/leads/third lead.fxp".to_string(),
        ),
    ];

    let resolved = resolve_display_patch_name(&pairs, "Pads/Factory Pad.fxp");

    assert_eq!(
        resolved.as_deref(),
        Some("patches_factory/Pads/Factory Pad.fxp")
    );
}

#[test]
fn resolve_display_patch_name_prefers_existing_prefixed_name() {
    let pairs = vec![(
        "patches_3rdparty/Leads/Third Lead.fxp".to_string(),
        "patches_3rdparty/leads/third lead.fxp".to_string(),
    )];

    let resolved = resolve_display_patch_name(&pairs, "patches_3rdparty/Leads/Third Lead.fxp");

    assert_eq!(
        resolved.as_deref(),
        Some("patches_3rdparty/Leads/Third Lead.fxp")
    );
}

#[test]
fn sort_patch_pairs_can_group_by_category_before_path() {
    let mut pairs = vec![
        (
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_factory/pad/super pad.fxp".to_string(),
        ),
        (
            "patches_3rdparty/john/lead/Great Lead.fxp".to_string(),
            "patches_3rdparty/john/lead/great lead.fxp".to_string(),
        ),
        (
            "patches_3rdparty/john/pad/Great Pad.fxp".to_string(),
            "patches_3rdparty/john/pad/great pad.fxp".to_string(),
        ),
        (
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_factory/lead/super lead.fxp".to_string(),
        ),
    ];

    sort_patch_pairs(&mut pairs, PatchSortOrder::Category);

    assert_eq!(
        pairs
            .into_iter()
            .map(|(display, _)| display)
            .collect::<Vec<_>>(),
        vec![
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_3rdparty/john/lead/Great Lead.fxp".to_string(),
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_3rdparty/john/pad/Great Pad.fxp".to_string(),
        ]
    );
}

#[test]
fn sort_patch_pairs_path_order_keeps_factory_before_thirdparty() {
    let mut pairs = vec![
        (
            "patches_3rdparty/john/lead/Great Lead.fxp".to_string(),
            "patches_3rdparty/john/lead/great lead.fxp".to_string(),
        ),
        (
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_factory/pad/super pad.fxp".to_string(),
        ),
        (
            "patches_3rdparty/john/pad/Great Pad.fxp".to_string(),
            "patches_3rdparty/john/pad/great pad.fxp".to_string(),
        ),
        (
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_factory/lead/super lead.fxp".to_string(),
        ),
    ];

    sort_patch_pairs(&mut pairs, PatchSortOrder::Path);

    assert_eq!(
        pairs
            .into_iter()
            .map(|(display, _)| display)
            .collect::<Vec<_>>(),
        vec![
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_3rdparty/john/lead/Great Lead.fxp".to_string(),
            "patches_3rdparty/john/pad/Great Pad.fxp".to_string(),
        ]
    );
}

#[test]
fn sort_patch_pairs_category_order_handles_vendorless_thirdparty_paths() {
    let mut pairs = vec![
        (
            "patches_3rdparty/lead/Great Lead.fxp".to_string(),
            "patches_3rdparty/lead/great lead.fxp".to_string(),
        ),
        (
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_factory/pad/super pad.fxp".to_string(),
        ),
        (
            "patches_3rdparty/pad/Great Pad.fxp".to_string(),
            "patches_3rdparty/pad/great pad.fxp".to_string(),
        ),
        (
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_factory/lead/super lead.fxp".to_string(),
        ),
    ];

    sort_patch_pairs(&mut pairs, PatchSortOrder::Category);

    assert_eq!(
        pairs
            .into_iter()
            .map(|(display, _)| display)
            .collect::<Vec<_>>(),
        vec![
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_3rdparty/lead/Great Lead.fxp".to_string(),
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_3rdparty/pad/Great Pad.fxp".to_string(),
        ]
    );
}
