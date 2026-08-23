use super::*;

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
    let cache = CacheFile {
        format_version: CACHE_FORMAT_VERSION,
        patches: vec!["Leads/LOUD Lead.fxp".to_string()],
        plugins: vec![plugin()],
        vvp_voicings: BTreeMap::new(),
        catalog_notes: vec!["notice".to_string()],
    };
    write_cache(&path, &cache).unwrap();

    let (loaded, vvp_voicings) = load_from(&path).unwrap().into_parts();

    assert_eq!(
        loaded.pairs(),
        &[(
            "Leads/LOUD Lead.fxp".to_string(),
            "leads/loud lead.fxp".to_string()
        )]
    );
    assert_eq!(loaded.catalog_notes(), &["notice"]);
    assert_eq!(loaded.catalog_plugins()[0].name, "Surge XT");
    assert!(vvp_voicings.is_empty());
    let _ = fs::remove_file(path);
}

#[test]
fn unsupported_version_is_rejected() {
    let path = temp_path("version");
    let cache = CacheFile {
        format_version: CACHE_FORMAT_VERSION + 1,
        patches: Vec::new(),
        plugins: vec![plugin()],
        vvp_voicings: BTreeMap::new(),
        catalog_notes: Vec::new(),
    };
    write_cache(&path, &cache).unwrap();

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
        vvp_voicings: BTreeMap::new(),
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
    let cache = CacheFile {
        format_version: CACHE_FORMAT_VERSION,
        patches: vec!["PD Wide.vvp".to_string()],
        plugins: vec![plugin()],
        vvp_voicings: BTreeMap::from([(
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
    let cache = CacheFile {
        format_version: CACHE_FORMAT_VERSION,
        patches: vec!["PD Wide.vvp".to_string()],
        plugins: vec![plugin()],
        vvp_voicings: BTreeMap::new(),
        catalog_notes: Vec::new(),
    };
    write_cache(&path, &cache).unwrap();

    let error = load_from(&path).err().unwrap();

    assert!(error.to_string().contains("VVP voicing"));
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

    let voicings = collect_vvp_voicings(&[plugin], &pairs);

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
