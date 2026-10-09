use super::*;

#[test]
fn cache_round_trip_distinguishes_known_unknown_and_empty_drum_notes() {
    let path = temp_path("drum_notes");
    let mut known = cached_patch("known.fxp", 12);
    known.measurement.drum_kit = true;
    known.measurement.drum_kit_notes = Some(vec![36, 42, 70]);
    known.measurement.drum_kit_note_names =
        vec![(36, "Kick".to_string()), (70, "Taiko Drum 1".to_string())];
    known.measurement.drum_kit_one_shot_notes = vec![36, 42];
    let mut empty = cached_patch("empty.fxp", 13);
    empty.measurement.drum_kit = true;
    empty.measurement.drum_kit_notes = Some(Vec::new());
    let mut unknown = cached_patch("unknown.fxp", 14);
    unknown.measurement.drum_kit = true;
    let cache = CacheFile {
        format_version: CACHE_FORMAT_VERSION,
        patches: vec![known, empty, unknown],
        plugins: vec![plugin()],
        patch_voicings: BTreeMap::new(),
        catalog_notes: vec!["extraction failed".to_string()],
    };
    write_cache(&path, &cache).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for field in [
        "drum_kit_notes",
        "drum_kit_note_names",
        "drum_kit_one_shot_notes",
    ] {
        assert!(
            value["patches"][2].get(field).is_none(),
            "old catalog has no optional field: {field}"
        );
    }
    let (snapshot, _) = load_from(&path).unwrap().into_parts();
    let measurements = snapshot.load_measurements();
    assert_eq!(
        measurements["known.fxp"].drum_kit_notes,
        Some(vec![36, 42, 70])
    );
    assert_eq!(
        measurements["known.fxp"].drum_kit_note_names,
        vec![(36, "Kick".to_string()), (70, "Taiko Drum 1".to_string())]
    );
    assert_eq!(
        measurements["known.fxp"].drum_kit_one_shot_notes,
        vec![36, 42]
    );
    assert_eq!(measurements["empty.fxp"].drum_kit_notes, Some(vec![]));
    assert_eq!(measurements["unknown.fxp"].drum_kit_notes, None);
    assert!(measurements["unknown.fxp"].drum_kit);
    assert_eq!(measurements["unknown.fxp"].second_load_ms, Some(14));
    assert_eq!(snapshot.catalog_notes(), &["extraction failed"]);
    fs::remove_file(path).unwrap();
}

#[test]
fn reused_load_measurements_get_new_notes_and_failed_extraction_clears_old_notes() {
    let root = temp_path("drum_refresh").with_extension("");
    fs::create_dir_all(&root).unwrap();
    let path = root.join("kit.sfz");
    let plugin = CatalogPlugin {
        name: "Sforzando".to_string(),
        plugin_path: "X:/sforzando.clap".to_string(),
        plugin_id: Some(cmrt_runtime::SFORZANDO_PLUGIN_ID.to_string()),
        base: cmrt_runtime::PatchBase::Shared(root.to_string_lossy().into_owned()),
        dirs: vec![root.to_string_lossy().into_owned()],
        resolved_patches: None,
        source_notices: Vec::new(),
    };
    let mut measurements = BTreeMap::from([(
        "kit.sfz".to_string(),
        PatchLoadMeasurement {
            second_load_ms: Some(12),
            drum_kit_notes: Some(vec![99]),
            drum_kit_note_names: vec![(99, "stale".to_string())],
            drum_kit_one_shot_notes: vec![99],
            ..Default::default()
        },
    )]);
    for (second_key, expected, label) in [(42, vec![36, 42], "Snare"), (70, vec![36, 70], "Rim")] {
        fs::write(
            &path,
            format!("<region> sample=kick.wav key=36 loop_mode=one_shot\n<region> sample=snare.wav key={second_key} region_label={label}"),
        )
        .unwrap();
        assert!(drum_kits::record(std::slice::from_ref(&plugin), &mut measurements).is_empty());
        assert_eq!(
            measurements["kit.sfz"].drum_kit_note_names,
            vec![(36, "kick".to_string()), (second_key, label.to_string())]
        );
        assert_eq!(measurements["kit.sfz"].drum_kit_notes, Some(expected));
        assert_eq!(measurements["kit.sfz"].drum_kit_one_shot_notes, vec![36]);
        assert!(measurements["kit.sfz"].drum_kit);
        assert_eq!(measurements["kit.sfz"].second_load_ms, Some(12));
    }
    fs::write(&path,"<region> sample=kick.wav key=36\n<region> sample=snare.wav key=70\n#include \"missing.sfz\"").unwrap();
    let failures = drum_kits::record(std::slice::from_ref(&plugin), &mut measurements);
    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("kit.sfz") && failures[0].contains("missing.sfz"));
    assert_eq!(measurements["kit.sfz"].drum_kit_notes, None);
    assert!(measurements["kit.sfz"].drum_kit_note_names.is_empty());
    assert!(measurements["kit.sfz"].drum_kit_one_shot_notes.is_empty());
    assert!(measurements["kit.sfz"].drum_kit);
    fs::write(&path, "<region> sample=lead.wav lokey=0 hikey=127").unwrap();
    assert!(drum_kits::record(&[plugin], &mut measurements).is_empty());
    assert!(!measurements["kit.sfz"].drum_kit);
    assert_eq!(measurements["kit.sfz"].drum_kit_notes, None);
    fs::remove_dir_all(root).unwrap();
}

/// Read installed files through the catalog's saved bases, without overwriting the real cache
/// or starting a patch-load measurement server.
#[test]
#[ignore = "real catalog and libraries required: set CMRT_TEST_CATALOG_CACHE"]
fn installed_catalog_kit_notes_extract_completely() {
    let path = std::env::var("CMRT_TEST_CATALOG_CACHE").expect("real catalog path");
    let cache: CacheFile = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let kits: Vec<_> = cache
        .patches
        .iter()
        .filter(|patch| patch.measurement.drum_kit)
        .map(|patch| patch.audio.reference.display.clone())
        .collect();
    assert!(!kits.is_empty(), "the real catalog should contain kits");
    for patch in cache.patches.iter().filter(|p| !p.measurement.drum_kit) {
        assert!(
            patch.measurement.drum_kit_note_names.is_empty(),
            "a non-kit patch keeps names: {}",
            patch.audio.reference.display
        );
    }
    let saved: BTreeMap<_, _> = cache
        .patches
        .iter()
        .filter(|patch| patch.measurement.drum_kit)
        .map(|patch| {
            let measurement = &patch.measurement;
            let saved = (
                measurement.drum_kit_notes.clone(),
                measurement.drum_kit_note_names.clone(),
                measurement.drum_kit_one_shot_notes.clone(),
            );
            (patch.audio.reference.display.clone(), saved)
        })
        .collect();
    let mut measurements: BTreeMap<_, _> = cache
        .patches
        .into_iter()
        .map(|patch| (patch.audio.reference.display, patch.measurement))
        .collect();
    let plugins: Vec<_> = cache.plugins.into_iter().map(CatalogPlugin::from).collect();
    let failures = drum_kits::record(&plugins, &mut measurements);
    assert!(failures.is_empty(), "{failures:?}");
    for kit in &kits {
        let measurement = &measurements[kit];
        assert!(measurement.drum_kit, "kit classification changed: {kit}");
        let notes = measurement
            .drum_kit_notes
            .as_ref()
            .unwrap_or_else(|| panic!("kit notes are unknown: {kit}"));
        assert!(notes.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(notes.iter().all(|note| *note <= 127));
        let names = &measurement.drum_kit_note_names;
        assert!(names.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(names
            .iter()
            .all(|(note, _)| notes.binary_search(note).is_ok()));
        let one_shot = &measurement.drum_kit_one_shot_notes;
        assert!(one_shot
            .iter()
            .all(|note| notes.binary_search(note).is_ok()));
        let fresh = (
            measurement.drum_kit_notes.clone(),
            names.clone(),
            one_shot.clone(),
        );
        assert_eq!(
            saved[kit], fresh,
            "the saved catalog is stale, rebuild it: {kit}"
        );
        eprintln!("{kit}: {notes:?}");
        eprintln!("  names {}/{}: {names:?}", names.len(), notes.len());
        eprintln!(
            "  one-shot {}/{}: {one_shot:?}",
            one_shot.len(),
            notes.len()
        );
    }
    eprintln!("extracted installed kits={}", kits.len());
}
