use super::*;

fn level(track: usize, rms_db: f32, peak_db: f32) -> TrackLevel {
    TrackLevel {
        track,
        rms_db,
        peak_db,
    }
}

/// peak が天井へ届かない、測定値として素直な track。
fn quiet_peak_level(track: usize, rms_db: f32) -> TrackLevel {
    level(track, rms_db, rms_db + 3.0)
}

#[test]
fn constant_amplitude_is_measured_as_rms_and_peak() {
    let measured = measure_track_level(2, &[0.5; 128]).expect("0.5 定数は無音ではない");
    assert_eq!(measured.track, 2);
    assert!((measured.rms_db - (-6.0206)).abs() < 0.01);
    assert!((measured.peak_db - (-6.0206)).abs() < 0.01);
}

#[test]
fn silence_and_near_silence_are_not_measured() {
    assert_eq!(measure_track_level(2, &[]), None);
    assert_eq!(measure_track_level(2, &[0.0; 128]), None);
    // -80dBFS RMS。残響の尻尾を「静かな patch」と誤認しないための無音ゲート。
    assert_eq!(measure_track_level(2, &[1.0e-4; 128]), None);
}

#[test]
fn unmeasured_tracks_stay_at_zero_db() {
    assert_eq!(auto_trim_volumes_db(&[], 4), vec![0; 4]);
    // track 2 だけ測れた場合、残りは 0dB のまま。
    let volumes_db = auto_trim_volumes_db(&[quiet_peak_level(2, -20.0)], 4);
    assert_eq!(volumes_db[0], 0);
    assert_eq!(volumes_db[1], 0);
    assert_eq!(volumes_db[3], 0);
}

#[test]
fn evenly_balanced_tracks_are_left_alone() {
    let volumes_db =
        auto_trim_volumes_db(&[quiet_peak_level(2, -20.0), quiet_peak_level(3, -20.0)], 4);
    assert_eq!(volumes_db, vec![0, 0, 0, 0]);
}

#[test]
fn quietest_track_becomes_the_zero_db_reference() {
    // 補正量がクランプへ届かない範囲では、一番小さい track がちょうど 0dB になり、
    // 他の track はすべてそこから下がる。
    let volumes_db =
        auto_trim_volumes_db(&[quiet_peak_level(2, -10.0), quiet_peak_level(3, -14.0)], 4);
    assert_eq!(volumes_db[2], -3);
    assert_eq!(volumes_db[3], 0);
}

#[test]
fn no_track_is_boosted_above_zero_db() {
    let volumes_db = auto_trim_volumes_db(
        &[
            quiet_peak_level(2, -10.0),
            quiet_peak_level(3, -20.0),
            quiet_peak_level(4, -30.0),
        ],
        5,
    );
    assert!(volumes_db.iter().all(|&volume_db| volume_db <= 0));
}

#[test]
fn one_tiny_track_does_not_drag_the_whole_mix_down() {
    // -45dB の track を 0dB 基準にすると、クランプが無ければ最大 track は -35dB まで沈む。
    // 補正量を先にクランプすることで -15dB で止まる。
    let volumes_db = auto_trim_volumes_db(
        &[
            quiet_peak_level(2, -10.0),
            quiet_peak_level(3, -20.0),
            quiet_peak_level(4, -30.0),
            quiet_peak_level(5, -45.0),
        ],
        6,
    );
    assert_eq!(&volumes_db[2..], &[-15, -3, 0, 0]);
}

#[test]
fn transient_heavy_track_is_pushed_below_the_peak_ceiling() {
    // RMS だけ見れば据え置き（0dB）だが、peak が 0dBFS なので 1 step 下げる。
    let volumes_db = auto_trim_volumes_db(&[level(2, -20.0, 0.0)], 3);
    assert_eq!(volumes_db[2], -3);
}

#[test]
fn results_stay_inside_the_mixer_range_and_on_its_step_grid() {
    let volumes_db = auto_trim_volumes_db(
        &[
            level(2, -3.0, 0.0),
            quiet_peak_level(3, -20.0),
            quiet_peak_level(4, -55.0),
        ],
        5,
    );
    for volume_db in volumes_db {
        assert!((MIXER_MIN_DB..=0).contains(&volume_db));
        assert_eq!(volume_db % MIXER_STEP_DB, 0);
    }
}
