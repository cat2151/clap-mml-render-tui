use super::*;

fn test_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cmrt_catalog_sources_{label}_{}.json",
        std::process::id()
    ))
}

#[test]
fn writes_the_server_startup_projection_without_patch_rows() {
    let path = test_path("projection");
    let plugin = CatalogPlugin {
        name: "Sforzando".to_string(),
        plugin_path: "C:/CLAP/sforzando.clap".to_string(),
        plugin_id: Some("com.example.sforzando".to_string()),
        base: Some("C:/SFZ".to_string()),
        dirs: vec!["C:/SFZ/User".to_string(), "C:/SFZ/Banks".to_string()],
        resolved_patches: Some(vec![PathBuf::from("C:/SFZ/User/Piano.sfz")]),
        source_notices: vec!["12 files excluded".to_string()],
    };

    write(&path, &[plugin]).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let _ = std::fs::remove_file(&path);

    assert_eq!(value["format_version"], FORMAT_VERSION);
    assert_eq!(value["plugins"][0]["name"], "Sforzando");
    assert_eq!(value["plugins"][0]["dirs"].as_array().unwrap().len(), 2);
    assert!(value["plugins"][0].get("resolved_patches").is_none());
    assert!(value.get("patches").is_none());
}
