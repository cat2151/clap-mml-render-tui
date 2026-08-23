use super::*;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

fn plugin() -> CachedPlugin {
    CachedPlugin {
        name: "Surge XT".to_string(),
        plugin_path: "C:/Surge XT.clap".to_string(),
        plugin_id: Some(cmrt_runtime::SURGE_XT_PLUGIN_ID.to_string()),
        base: Some("C:/patches".to_string()),
        dirs: vec!["C:/patches".to_string()],
        source_notices: Vec::new(),
        patch_roles: PatchRoles::default(),
    }
}

fn catalog_metadata_plugin() -> CachedPlugin {
    CachedPlugin {
        name: "catalog-metadata-plugin".to_string(),
        plugin_path: "C:/catalog-metadata.clap".to_string(),
        plugin_id: Some(cmrt_runtime::VAPORIZER2_PLUGIN_ID.to_string()),
        ..plugin()
    }
}

fn cached_patch(display: &str, second_load_ms: u64) -> CachedPatch {
    cached_patch_for(&plugin(), display, second_load_ms)
}

fn cached_patch_for(plugin: &CachedPlugin, display: &str, second_load_ms: u64) -> CachedPatch {
    let info = cmrt_core::AudioPluginInfo::new(
        plugin.name.clone(),
        plugin.plugin_path.clone(),
        plugin.plugin_id.clone(),
        plugin.base.clone(),
    );
    CachedPatch {
        audio: info.describe_patch(display, None),
        measurement: PatchLoadMeasurement {
            second_load_ms: Some(second_load_ms),
            ..PatchLoadMeasurement::default()
        },
    }
}

fn temp_path(label: &str) -> PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cmrt_patch_catalog_cache_{label}_{}_{}.json",
        std::process::id(),
        suffix
    ))
}

#[test]
fn cache_round_trip_rebuilds_lowercase_search_value() {
    let path = temp_path("round_trip");
    let mut patch = cached_patch("Leads/LOUD Lead.fxp", 234);
    patch.measurement.first_load_error = Some("warmup failed".to_string());
    let cache = CacheFile {
        format_version: CACHE_FORMAT_VERSION,
        patches: vec![patch],
        plugins: vec![plugin()],
        patch_voicings: BTreeMap::new(),
        catalog_notes: vec!["notice".to_string()],
    };
    write_cache(&path, &cache).unwrap();

    let (loaded, patch_voicings) = load_from(&path).unwrap().into_parts();

    assert_eq!(
        loaded.pairs(),
        &[(
            "Leads/LOUD Lead.fxp".to_string(),
            "leads/loud lead.fxp".to_string()
        )]
    );
    assert_eq!(loaded.catalog_notes(), &["notice"]);
    assert_eq!(loaded.catalog_plugins()[0].name, "Surge XT");
    assert_eq!(
        loaded.audio_patches()[0].reference.display,
        "Leads/LOUD Lead.fxp"
    );
    assert_eq!(
        loaded.audio_patches()[0].reference.plugin,
        cmrt_core::PluginKey::from_identity(
            Some(cmrt_runtime::SURGE_XT_PLUGIN_ID),
            "C:/Surge XT.clap"
        )
    );
    assert_eq!(loaded.audio_patches()[0].sort.category, "Leads");
    assert_eq!(
        loaded.load_measurements()["Leads/LOUD Lead.fxp"].second_load_ms,
        Some(234)
    );
    assert_eq!(
        loaded.load_measurements()["Leads/LOUD Lead.fxp"]
            .first_load_error
            .as_deref(),
        Some("warmup failed")
    );
    assert!(patch_voicings.is_empty());
    let _ = fs::remove_file(path);
}

#[test]
fn cache_rejects_a_patch_whose_plugin_key_does_not_match_the_catalog() {
    let path = temp_path("wrong_plugin_key");
    let mut patch = cached_patch("Leads/Lead.fxp", 12);
    patch.audio.reference.plugin = cmrt_core::PluginKey::from_identity(Some("another.plugin"), "");
    let cache = CacheFile {
        format_version: CACHE_FORMAT_VERSION,
        patches: vec![patch],
        plugins: vec![plugin()],
        patch_voicings: BTreeMap::new(),
        catalog_notes: Vec::new(),
    };
    write_cache(&path, &cache).unwrap();

    let error = load_from(&path).err().unwrap();

    assert!(error.to_string().contains("plugin key"), "{error}");
    let _ = fs::remove_file(path);
}

#[test]
fn unsupported_version_is_rejected() {
    let path = temp_path("version");
    let cache = CacheFile {
        format_version: CACHE_FORMAT_VERSION + 1,
        patches: Vec::new(),
        plugins: vec![plugin()],
        patch_voicings: BTreeMap::new(),
        catalog_notes: Vec::new(),
    };
    write_cache(&path, &cache).unwrap();

    let error = load_from(&path).err().unwrap();

    assert!(error.to_string().contains("format version"));
    let _ = fs::remove_file(path);
}

#[test]
fn legacy_string_patch_cache_reports_the_version_before_decoding_entries() {
    let path = temp_path("legacy_version");
    fs::write(
        &path,
        br#"{"format_version":2,"patches":["Lead.fxp"],"plugins":[]}"#,
    )
    .unwrap();

    let error = load_from(&path).err().unwrap();

    assert!(error.to_string().contains("format version"));
    let _ = fs::remove_file(path);
}

#[test]
fn broken_json_is_rejected() {
    let path = temp_path("broken");
    fs::write(&path, b"not json").unwrap();

    assert!(load_from(&path).is_err());

    let _ = fs::remove_file(path);
}

#[test]
fn plugin_with_empty_path_is_rejected() {
    let path = temp_path("empty_plugin_path");
    let mut empty_path_plugin = plugin();
    empty_path_plugin.plugin_path.clear();
    let cache = CacheFile {
        format_version: CACHE_FORMAT_VERSION,
        patches: Vec::new(),
        plugins: vec![empty_path_plugin],
        patch_voicings: BTreeMap::new(),
        catalog_notes: Vec::new(),
    };
    write_cache(&path, &cache).unwrap();

    let error = load_from(&path).err().unwrap();

    assert!(error.to_string().contains("plugin_path"));
    let _ = fs::remove_file(path);
}

#[test]
fn vvp_voicing_round_trips_without_reading_the_preset() {
    let path = temp_path("vvp_round_trip");
    let metadata_plugin = catalog_metadata_plugin();
    let cache = CacheFile {
        format_version: CACHE_FORMAT_VERSION,
        patches: vec![cached_patch_for(&metadata_plugin, "PD Wide.vvp", 12)],
        plugins: vec![metadata_plugin],
        patch_voicings: BTreeMap::from([(
            "PD Wide.vvp".to_string(),
            cmrt_realtime_play::PatchVoicing::Poly,
        )]),
        catalog_notes: Vec::new(),
    };
    write_cache(&path, &cache).unwrap();

    let (_, voicings) = load_from(&path).unwrap().into_parts();

    assert_eq!(
        voicings.get("PD Wide.vvp"),
        Some(&cmrt_realtime_play::PatchVoicing::Poly)
    );
    let _ = fs::remove_file(path);
}

#[test]
fn vvp_patch_without_persisted_voicing_is_rejected() {
    let path = temp_path("missing_vvp_voicing");
    let metadata_plugin = catalog_metadata_plugin();
    let cache = CacheFile {
        format_version: CACHE_FORMAT_VERSION,
        patches: vec![cached_patch_for(&metadata_plugin, "PD Wide.vvp", 12)],
        plugins: vec![metadata_plugin],
        patch_voicings: BTreeMap::new(),
        catalog_notes: Vec::new(),
    };
    write_cache(&path, &cache).unwrap();

    let error = load_from(&path).err().unwrap();

    assert!(error.to_string().contains("adapter voicing"));
    let _ = fs::remove_file(path);
}

#[test]
fn cache_build_reads_vvp_poly_mode_once() {
    let root = temp_path("collect_vvp").with_extension("");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("PD Wide.vvp"),
        br#"<VASTvaporizer2><PARAM id="m_uPolyMode" text="Poly16"/></VASTvaporizer2>"#,
    )
    .unwrap();
    fs::write(
        root.join("LD Mono.vvp"),
        br#"<VASTvaporizer2><PARAM id="m_uPolyMode" text="Mono"/></VASTvaporizer2>"#,
    )
    .unwrap();
    let plugin = CatalogPlugin {
        name: "Vaporizer2".to_string(),
        plugin_path: "VASTvaporizer2.clap".to_string(),
        plugin_id: Some(cmrt_runtime::VAPORIZER2_PLUGIN_ID.to_string()),
        base: Some(root.to_string_lossy().into_owned()),
        dirs: vec![root.to_string_lossy().into_owned()],
        resolved_patches: None,
        source_notices: Vec::new(),
        patch_roles: PatchRoles::default(),
    };
    let pairs = vec![
        ("PD Wide.vvp".to_string(), "pd wide.vvp".to_string()),
        ("LD Mono.vvp".to_string(), "ld mono.vvp".to_string()),
        ("PD Missing.vvp".to_string(), "pd missing.vvp".to_string()),
    ];

    let plugins = [plugin];
    let patches = describe_patches(&plugins, &pairs).unwrap();
    let voicings = collect_patch_voicings(&plugins, &patches);

    assert_eq!(
        voicings.get("PD Wide.vvp"),
        Some(&cmrt_realtime_play::PatchVoicing::Poly)
    );
    assert_eq!(
        voicings.get("LD Mono.vvp"),
        Some(&cmrt_realtime_play::PatchVoicing::Mono)
    );
    assert_eq!(
        voicings.get("PD Missing.vvp"),
        Some(&cmrt_realtime_play::PatchVoicing::Unknown)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn every_patch_loads_on_instance_zero_then_instance_one() {
    let pairs = vec![
        ("A.fxp".to_string(), "a.fxp".to_string()),
        ("B.fxp".to_string(), "b.fxp".to_string()),
        ("C.fxp".to_string(), "c.fxp".to_string()),
    ];
    let base = Instant::now();
    let mut times = VecDeque::from([
        base,
        base + Duration::from_millis(234),
        base + Duration::from_secs(1),
        base + Duration::from_millis(1_999),
        base + Duration::from_secs(2),
        base + Duration::from_millis(2_007),
    ]);
    let mut calls = Vec::new();
    let measurements = measure_patch_loads(
        &pairs,
        |instance, patch| {
            calls.push((instance, patch.to_string()));
            match (instance, patch) {
                (0, "A.fxp") => anyhow::bail!("warmup failed"),
                (1, "B.fxp") => anyhow::bail!("second failed"),
                _ => Ok(()),
            }
        },
        || times.pop_front().unwrap(),
        |_, _| {},
        |_, _, _| {},
    );

    assert_eq!(
        calls,
        vec![
            (0, "A.fxp".to_string()),
            (1, "A.fxp".to_string()),
            (0, "B.fxp".to_string()),
            (1, "B.fxp".to_string()),
            (0, "C.fxp".to_string()),
            (1, "C.fxp".to_string()),
        ]
    );
    assert_eq!(measurements["A.fxp"].second_load_ms, Some(234));
    assert!(measurements["A.fxp"]
        .first_load_error
        .as_deref()
        .unwrap()
        .contains("warmup failed"));
    assert_eq!(measurements["B.fxp"].second_load_ms, None);
    assert!(measurements["B.fxp"]
        .second_load_error
        .as_deref()
        .unwrap()
        .contains("second failed"));
    assert_eq!(measurements["C.fxp"].second_load_ms, Some(7));
}

#[test]
fn cache_rejects_a_patch_without_a_second_load_result() {
    let path = temp_path("missing_load_result");
    let cache = CacheFile {
        format_version: CACHE_FORMAT_VERSION,
        patches: vec![CachedPatch {
            measurement: PatchLoadMeasurement::default(),
            ..cached_patch("Lead.fxp", 0)
        }],
        plugins: vec![plugin()],
        patch_voicings: BTreeMap::new(),
        catalog_notes: Vec::new(),
    };
    write_cache(&path, &cache).unwrap();

    let error = load_from(&path).err().unwrap();

    assert!(error.to_string().contains("2回目のload計測結果"));
    let _ = fs::remove_file(path);
}

#[test]
fn eta_uses_the_average_of_completed_patches_for_the_remaining_count() {
    assert_eq!(
        estimate_eta(Duration::from_secs(503), 2, 5),
        Duration::from_millis(754_500)
    );
    assert_eq!(estimate_eta(Duration::from_secs(503), 5, 5), Duration::ZERO);
    assert_eq!(estimate_eta(Duration::from_secs(503), 0, 5), Duration::ZERO);
}

#[test]
fn eta_format_uses_total_minutes_and_two_digit_seconds() {
    assert_eq!(format_eta(Duration::from_secs(5)), "0分05秒");
    assert_eq!(format_eta(Duration::from_secs(754)), "12分34秒");
    assert_eq!(format_eta(Duration::from_secs(7_445)), "124分05秒");
}
