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
fn empty_and_occupied_cells_round_trip_in_sparse_format() {
    let dir = temp_dir("cmrt-loop-track-grid");
    let path = dir.join("loop_browser").join("track_grid.toml");
    let wav = LoopWavId::new(Path::new("/loops"), Path::new("Pack/Bass/a.wav"));
    let grid = vec![
        vec![
            None,
            Some(LoopTrackClip {
                wav,
                span_measures: 2,
            }),
            None,
        ],
        vec![None, None, None],
    ];

    save_to(&path, &grid).unwrap();

    assert_eq!(load_from(&path).unwrap().grid, grid);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("track_count = 2"));
    assert!(text.contains("measure_count = 3"));
    assert!(text.contains("span_measures = 2"));
    assert_eq!(text.matches("[[cells]]").count(), 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn missing_file_loads_one_empty_cell() {
    let path = temp_dir("cmrt-loop-track-grid-missing").join("track_grid.toml");

    assert_eq!(load_from(&path).unwrap().grid, default_track_grid());
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
    };
    assert!(loaded.needs_migration);

    let (grid, changed) = reflow_with_spans(&loaded.grid, |_| Some(2));
    assert!(changed);
    assert_eq!(grid[0].len(), 4);
    assert_eq!(grid[0][0].as_ref().unwrap().wav.relative, "a.wav");
    assert_eq!(grid[0][2].as_ref().unwrap().wav.relative, "b.wav");
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
