use super::*;

fn temp_path(label: &str) -> std::path::PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cmrt_previous_measurements_{label}_{}_{}.json",
        std::process::id(),
        suffix
    ))
}

fn pair(display: &str) -> (String, String) {
    (display.to_string(), display.to_lowercase())
}

#[test]
fn only_patches_with_a_second_load_time_count_as_measured() {
    let path = temp_path("mixed");
    let json = serde_json::json!({
        "format_version": 3,
        "patches": [
            {"audio": {"reference": {"display": "Surge XT/Lead"}}, "second_load_ms": 120},
            {"audio": {"reference": {"display": "Surge XT/Broken"}},
             "second_load_ms": null, "second_load_error": "timeout"},
            {"audio": {}, "second_load_ms": 5}
        ]
    });
    fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();

    let previous = read(&path);
    let _ = fs::remove_file(&path);
    let (measured, unmeasured) =
        partition(&[pair("Surge XT/Lead"), pair("Surge XT/Broken")], previous);

    assert_eq!(measured.keys().collect::<Vec<_>>(), vec!["Surge XT/Lead"]);
    assert_eq!(measured["Surge XT/Lead"].second_load_ms, Some(120));
    assert_eq!(unmeasured, vec![pair("Surge XT/Broken")]);
}

#[test]
fn missing_or_broken_cache_means_everything_is_unmeasured() {
    let missing = temp_path("missing");
    assert!(read(&missing).is_empty());

    let broken = temp_path("broken");
    fs::write(&broken, b"{not json").unwrap();
    let previous = read(&broken);
    let _ = fs::remove_file(&broken);
    assert!(previous.is_empty());
}

#[test]
fn partition_reuses_measured_patches_and_lists_the_rest_in_catalog_order() {
    let previous = BTreeMap::from([
        (
            "Surge XT/Lead".to_string(),
            PatchLoadMeasurement {
                second_load_ms: Some(120),
                ..PatchLoadMeasurement::default()
            },
        ),
        (
            "Surge XT/Removed".to_string(),
            PatchLoadMeasurement {
                second_load_ms: Some(50),
                ..PatchLoadMeasurement::default()
            },
        ),
    ]);
    let pairs = vec![
        pair("Surge XT/New B"),
        pair("Surge XT/Lead"),
        pair("Surge XT/New A"),
    ];

    let (measured, unmeasured) = partition(&pairs, previous);

    assert_eq!(measured.keys().collect::<Vec<_>>(), vec!["Surge XT/Lead"]);
    assert_eq!(
        unmeasured,
        vec![pair("Surge XT/New B"), pair("Surge XT/New A")]
    );
}
