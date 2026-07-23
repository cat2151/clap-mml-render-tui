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
fn explicit_and_prev_cells_round_trip_without_copying_the_wav_path() {
    let dir = temp_dir("cmrt-loop-track-grid");
    let path = dir.join("loop_browser").join("track_grid.toml");
    let wav = LoopWavId::new(Path::new("/loops"), Path::new("Pack/Bass/a.wav"));
    let explicit_grid = vec![
        vec![None, Some(LoopTrackClip::explicit(wav.clone(), 2)), None],
        vec![Some(LoopTrackClip::explicit(wav, 1)), None, None],
    ];
    let (grid, changed) = normalize_previous_markers(&explicit_grid);
    assert!(changed);

    save_to(&path, &grid, &[-3, 6]).unwrap();

    let loaded = load_from(&path).unwrap();
    assert_eq!(loaded.grid, grid);
    assert_eq!(loaded.track_volumes_db, vec![-3, 6]);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("track_count = 2"));
    assert!(text.contains("measure_count = 3"));
    assert!(text.contains("track_volumes_db = ["));
    assert!(text.contains("span_measures = 2"));
    assert_eq!(text.matches("[[cells]]").count(), 5);
    assert_eq!(text.matches("kind = \"prev\"").count(), 3);
    assert_eq!(text.matches("root = ").count(), 2);
    assert!(text.contains("source_measure = 1"));
    assert!(text.contains("source_measure = 0"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn missing_file_loads_one_empty_cell() {
    let path = temp_dir("cmrt-loop-track-grid-missing").join("track_grid.toml");

    assert_eq!(load_from(&path).unwrap().grid, default_track_grid());
    assert_eq!(load_from(&path).unwrap().track_volumes_db, vec![0]);
}

#[test]
fn version_one_loads_for_migration_and_reflows_overlaps_to_the_right() {
    let text = "version = 1\ntrack_count = 1\nmeasure_count = 2\n\n\
        [[cells]]\ntrack = 0\nmeasure = 0\nroot = '/loops'\nrelative = 'a.wav'\n\n\
        [[cells]]\ntrack = 0\nmeasure = 1\nroot = '/loops'\nrelative = 'b.wav'\n";
    let stored: StoredTrackGrid = toml::from_str(text).unwrap();
    let loaded = LoadedTrackGrid {
        needs_migration: stored.version == 1,
        grid: stored.into_grid().unwrap(),
        track_volumes_db: vec![0],
    };
    assert!(loaded.needs_migration);

    let (grid, changed) = reflow_with_spans(&loaded.grid, |_| Some(2));
    assert!(changed);
    assert_eq!(grid[0].len(), 4);
    assert_eq!(grid[0][0].as_ref().unwrap().wav.relative, "a.wav");
    assert_eq!(grid[0][2].as_ref().unwrap().wav.relative, "b.wav");
}

#[test]
fn version_two_loads_with_zero_db_and_is_marked_for_migration() {
    let dir = temp_dir("cmrt-loop-track-grid-v2");
    let path = dir.join("track_grid.toml");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&path, "version = 2\ntrack_count = 2\nmeasure_count = 1\n").unwrap();

    let loaded = load_from(&path).unwrap();

    assert!(loaded.needs_migration);
    assert_eq!(loaded.track_volumes_db, vec![0, 0]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn version_three_shape_normalizes_two_measure_tracks_to_the_longest_axis() {
    let wav = |name: &str, span| {
        Some(LoopTrackClip::explicit(
            LoopWavId::new(Path::new("/loops"), Path::new(name)),
            span,
        ))
    };
    let grid = vec![
        vec![wav("long.wav", 4), None, None, None],
        vec![wav("two-a.wav", 2), None, None, None],
        vec![wav("two-b.wav", 2), None, None, None],
        vec![wav("one.wav", 1), None, None, None],
    ];

    let (normalized, changed) = normalize_previous_markers(&grid);

    assert!(changed);
    assert!(normalized[0][0]
        .as_ref()
        .is_some_and(|clip| !clip.is_previous()));
    assert!(normalized[0][1..].iter().all(Option::is_none));
    for track in normalized.iter().take(3).skip(1) {
        let marker = track[2].as_ref().unwrap();
        assert!(marker.is_previous());
        assert_eq!(marker.span_measures, 2);
        assert_eq!(marker.previous_source_measure, Some(0));
        assert!(track[3].is_none());
    }
    assert!(normalized[3][1..]
        .iter()
        .all(|cell| cell.as_ref().is_some_and(LoopTrackClip::is_previous)));
}

#[test]
fn version_three_file_migrates_to_persisted_prev_markers() {
    let dir = temp_dir("cmrt-loop-track-grid-v3");
    let path = dir.join("track_grid.toml");
    std::fs::create_dir_all(&dir).unwrap();
    let mut text =
        "version = 3\ntrack_count = 4\nmeasure_count = 4\ntrack_volumes_db = [0, 0, 0, 0]\n"
            .to_string();
    for (track, span) in [(0, 4), (1, 2), (2, 2), (3, 1)] {
        text.push_str(&format!(
            "\n[[cells]]\ntrack = {track}\nmeasure = 0\nspan_measures = {span}\nroot = '/loops'\nrelative = 'track-{track}.wav'\n"
        ));
    }
    std::fs::write(&path, text).unwrap();

    let loaded = load_from(&path).unwrap();
    assert!(loaded.needs_migration);
    let normalized = normalize_previous_markers(&loaded.grid).0;
    save_to(&path, &normalized, &loaded.track_volumes_db).unwrap();

    let migrated_text = std::fs::read_to_string(&path).unwrap();
    assert!(migrated_text.starts_with("version = 4"));
    assert_eq!(migrated_text.matches("kind = \"prev\"").count(), 5);
    assert_eq!(load_from(&path).unwrap().grid, normalized);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn normalization_crops_the_last_prev_marker_at_the_axis_boundary() {
    let long = LoopWavId::new(Path::new("/loops"), Path::new("long.wav"));
    let three = LoopWavId::new(Path::new("/loops"), Path::new("three.wav"));
    let grid = vec![
        vec![Some(LoopTrackClip::explicit(long, 4)), None, None, None],
        vec![Some(LoopTrackClip::explicit(three, 3)), None, None, None],
    ];

    let (normalized, _) = normalize_previous_markers(&grid);

    let cropped = normalized[1][3].as_ref().unwrap();
    assert!(cropped.is_previous());
    assert_eq!(cropped.span_measures, 1);
    assert_eq!(cropped.previous_source_measure, Some(0));
}

#[test]
fn normalization_fills_leading_space_from_the_tracks_last_explicit_clip() {
    let wav = LoopWavId::new(Path::new("/loops"), Path::new("late.wav"));
    let grid = vec![
        vec![None, Some(LoopTrackClip::explicit(wav, 1)), None],
        vec![
            Some(LoopTrackClip::explicit(
                LoopWavId::new(Path::new("/loops"), Path::new("axis.wav")),
                3,
            )),
            None,
            None,
        ],
    ];

    let (normalized, _) = normalize_previous_markers(&grid);

    assert!(normalized[0][0]
        .as_ref()
        .is_some_and(LoopTrackClip::is_previous));
    assert!(normalized[0][2]
        .as_ref()
        .is_some_and(LoopTrackClip::is_previous));
}

#[test]
fn prev_marker_without_an_explicit_source_is_rejected() {
    let text = "version = 4\ntrack_count = 1\nmeasure_count = 1\n\n\
        [[cells]]\ntrack = 0\nmeasure = 0\nkind = 'prev'\nsource_measure = 0\n";
    let stored: StoredTrackGrid = toml::from_str(text).unwrap();

    assert!(stored.into_grid().is_err());
}

#[test]
fn duplicate_or_out_of_range_cells_are_rejected() {
    let wav = "root = '/loops'\nrelative = 'a.wav'\n";
    let duplicate = format!(
        "version = 1\ntrack_count = 1\nmeasure_count = 1\n\n[[cells]]\ntrack = 0\nmeasure = 0\n{wav}\n[[cells]]\ntrack = 0\nmeasure = 0\n{wav}"
    );
    let out_of_range = format!(
        "version = 1\ntrack_count = 1\nmeasure_count = 1\n\n[[cells]]\ntrack = 1\nmeasure = 0\n{wav}"
    );

    assert!(toml::from_str::<StoredTrackGrid>(&duplicate)
        .unwrap()
        .into_grid()
        .is_err());
    assert!(toml::from_str::<StoredTrackGrid>(&out_of_range)
        .unwrap()
        .into_grid()
        .is_err());
}
