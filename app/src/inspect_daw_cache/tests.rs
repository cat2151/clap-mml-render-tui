use super::*;

/// 先頭 `sounding` frame だけ一定振幅で鳴るステレオ信号。
fn tone(frames: usize, sounding: usize) -> Vec<f32> {
    (0..frames)
        .flat_map(|frame| {
            let value = if frame < sounding { 0.5 } else { 0.0 };
            [value, value]
        })
        .collect()
}

#[test]
fn sounding_end_is_last_frame_above_threshold() {
    assert_eq!(sounding_end_frames(&tone(100, 40)), 40);
    assert_eq!(sounding_end_frames(&tone(100, 0)), 0);
}

#[test]
fn window_rms_splits_only_the_measure_and_ignores_tail() {
    // 小節 64 frame、後ろに余韻 64 frame。前半 32 frame だけ鳴る。
    let envelope = window_rms(&tone(128, 32), 64);
    assert_eq!(envelope.len(), ENVELOPE_COLUMNS);
    assert!(envelope[..ENVELOPE_COLUMNS / 2].iter().all(|v| *v > 0.4));
    assert!(envelope[ENVELOPE_COLUMNS / 2..].iter().all(|v| *v == 0.0));
}

#[test]
fn envelope_digits_scale_to_zero_through_nine() {
    assert_eq!(envelope_digits(&[0.0, 0.5, 1.0], 1.0), "049");
    assert_eq!(envelope_digits(&[0.3], 0.0), "0");
}

#[test]
fn classify_flags_cache_that_stops_before_fresh_render() {
    assert_eq!(classify(false, 1000, 1000), CellVerdict::Ok);
    assert_eq!(classify(false, 1000, 200), CellVerdict::CacheCut);
    // 打楽器のように新しい render も短いなら、同じ長さの cache は正常。
    assert_eq!(classify(false, 200, 200), CellVerdict::Ok);
    assert_eq!(classify(true, 1000, 1000), CellVerdict::StaleHash);
}

fn cell_with_cache(path: &Path) -> DawCellRenderPlan {
    DawCellRenderPlan {
        grid_row: 1,
        display_track: 1,
        measure: 1,
        mml: String::new(),
        mml_hash: 0,
        saved_mml_hash: Some(0),
        cache_wav: Some(path.to_path_buf()),
    }
}

#[test]
fn delete_broken_removes_only_cut_caches() {
    let dir = std::env::temp_dir().join(format!(
        "cmrt-inspect-daw-cache-delete-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let cut = dir.join("cut.wav");
    let missing_hash = dir.join("stale.wav");
    let ok = dir.join("ok.wav");
    for path in [&cut, &missing_hash, &ok] {
        std::fs::write(path, b"wav").unwrap();
    }
    let cells = [
        cell_with_cache(&cut),
        cell_with_cache(&missing_hash),
        cell_with_cache(&ok),
    ];
    let problems = [
        (&cells[0], CellVerdict::CacheCut),
        (&cells[1], CellVerdict::StaleHash),
    ];

    delete_cut_caches(&problems).unwrap();

    assert!(!cut.exists());
    assert!(missing_hash.exists());
    assert!(ok.exists());
    std::fs::remove_dir_all(&dir).unwrap();
}
