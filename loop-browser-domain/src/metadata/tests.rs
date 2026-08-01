use super::*;

fn temp_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn category_keys_use_name_characters_then_unused_alphabet() {
    let categories = ["Guitar", "glock", "lead", "123", "gale"].map(str::to_string);
    let keys = category_keys(&categories)
        .into_iter()
        .map(|(key, _)| key)
        .collect::<Vec<_>>();
    assert_eq!(keys, ['g', 'l', 'e', 'a', 'b']);
}

#[test]
fn wav_inherits_the_closest_parent_category() {
    let mut metadata = LoopBrowserMetadata::default();
    metadata.toggle_category(
        &LoopDirId::new(Path::new("/loops"), Path::new("Pack")),
        "drum",
    );
    metadata.toggle_category(
        &LoopDirId::new(Path::new("/loops"), Path::new("Pack/Soft")),
        "spoken",
    );

    let kick = LoopWavId::new(Path::new("/loops"), Path::new("Pack/Hard/Kick.wav"));
    let voice = LoopWavId::new(Path::new("/loops"), Path::new("Pack/Soft/Voice.wav"));
    let outside = LoopWavId::new(Path::new("/other"), Path::new("Pack/Kick.wav"));
    assert_eq!(metadata.category_for_wav(&kick), Some("drum"));
    assert_eq!(metadata.category_for_wav(&voice), Some("spoken"));
    assert_eq!(metadata.category_for_wav(&outside), None);
}

#[test]
fn metadata_round_trip_and_rejects_bad_data() {
    let dir = temp_dir("cmrt-loop-browser-metadata");
    let path = dir.join("loop_browser.toml");
    let mut metadata = LoopBrowserMetadata::default();
    let id = LoopDirId::new(Path::new("/loops"), Path::new("Pack/Bass"));
    assert!(metadata.toggle_favorite(&id));
    metadata.toggle_category(&id, "bass");
    let wav = LoopWavId::new(Path::new("/loops"), Path::new("Pack/Bass/a.wav"));
    metadata.toggle_pad('c', &wav);
    save_to_path(&path, &metadata).unwrap();
    assert_eq!(load_from_path(&path).unwrap(), metadata);

    std::fs::write(&path, "version = 99").unwrap();
    assert!(load_from_path(&path).is_err());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn version_one_metadata_without_pads_stays_compatible() {
    let dir = temp_dir("cmrt-loop-browser-old-metadata");
    let path = dir.join("loop_browser.toml");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        &path,
        "version = 1\nfavorite_dirs = []\ncategory_assignments = []\n",
    )
    .unwrap();

    let metadata = load_from_path(&path).unwrap();

    assert!(metadata.pad_assignments.is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn metadata_rejects_invalid_or_duplicate_pads() {
    let wav = LoopWavId::new(Path::new("/loops"), Path::new("a.wav"));
    let mut metadata = LoopBrowserMetadata {
        pad_assignments: vec![
            LoopPadAssignment {
                pad: 'c',
                wav: wav.clone(),
            },
            LoopPadAssignment { pad: 'c', wav },
        ],
        ..LoopBrowserMetadata::default()
    };
    assert!(validate_metadata(&metadata).is_err());
    metadata.pad_assignments[1].pad = 'x';
    assert!(validate_metadata(&metadata).is_err());
}

#[test]
fn auto_random_round_trips_and_defaults_to_off_for_older_files() {
    let dir = temp_dir("cmrt-metadata-auto-random");
    let path = dir.join("loop_browser.toml");
    std::fs::create_dir_all(&dir).unwrap();
    // auto_random を知らない頃の TOML を読んでも既定の OFF になる。
    std::fs::write(
        &path,
        "version = 1\nfavorite_dirs = []\ncategory_assignments = []\n",
    )
    .unwrap();
    let mut metadata = load_from_path(&path).unwrap();
    assert!(!metadata.auto_random);

    assert!(metadata.toggle_auto_random());
    metadata.save_to(&path).unwrap();
    assert!(LoopBrowserMetadata::load_from(&path).unwrap().auto_random);

    assert!(!metadata.toggle_auto_random());
    let _ = std::fs::remove_dir_all(dir);
}
