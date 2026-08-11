use super::*;

#[test]
fn one_shot_analysis_is_excluded_from_target_and_stretch_limit_checks() {
    let mut browser = browser_with_direct_wavs(2);
    browser.wav_analyses[0].1 = browser.wav_analyses[0].1.into_one_shot();
    browser.wav_analyses[1].1.tempo.as_mut().unwrap().bpm = 160.0;
    let hit = LoopTrackClip::explicit(browser.wav_analyses[0].0.clone(), 1);
    let loop_clip = LoopTrackClip::explicit(browser.wav_analyses[1].0.clone(), 1);
    browser.track_grid[0] = vec![Some(hit.clone()), Some(loop_clip)];

    assert_eq!(browser.target_bpm().bpm, 128.0);
    assert_eq!(browser.playback_clip(&hit).bpm, None);
    assert!(!browser.clip_exceeds_time_ratio_limits(&hit, 37.0));
}

#[test]
fn a_one_shot_retriggers_only_at_its_own_power_of_two_boundary() {
    let mut browser = browser_with_spanning_wavs();
    browser.wav_analyses[1].1 = browser.wav_analyses[1].1.into_one_shot();
    // BPM120 4/4 なら 1 小節 2 秒。5 秒鳴る one-shot は 2.5 小節ぶんなので 4 小節へ切り上げる。
    browser.wav_analyses[1].1.duration_seconds = 5.0;
    let long_loop = browser.wav_analyses[0].0.clone();
    let one_shot = browser.wav_analyses[1].0.clone();
    browser.track_grid = vec![
        vec![Some(LoopTrackClip::explicit(one_shot, 1))],
        vec![Some(LoopTrackClip::explicit(long_loop, 8))],
    ];
    browser.normalize_track_grid();

    assert_eq!(browser.measure_seconds(), 2.0);
    assert!(browser.track_grid[0][0]
        .as_ref()
        .is_some_and(|clip| !clip.is_previous() && clip.span_measures == 4));

    let playback = browser.playback_grid();
    // 4 小節の区切りでだけ鳴らし直す。長い空白も、鳴り終わる前の重なりも作らない。
    assert!(playback[0][0]
        .as_ref()
        .is_some_and(LoopPlaybackClip::is_one_shot));
    assert!(playback[0][4]
        .as_ref()
        .is_some_and(LoopPlaybackClip::is_one_shot));
    assert!(playback[0][1..4].iter().all(Option::is_none));
    assert!(playback[0][5..].iter().all(Option::is_none));

    // 表示側は鳴り始めた小節を起点に波形を割り付ける（毎小節リプレイして見えないこと）。
    assert!(browser
        .clip_at(0, 2)
        .is_some_and(|(start, clip)| start == 0 && clip.span_measures == 4));
    assert!(browser
        .clip_at(0, 6)
        .is_some_and(|(start, clip)| start == 4 && clip.is_previous()));
}

#[test]
fn a_one_shot_longer_than_the_loops_extends_the_grid_axis() {
    let mut browser = browser_with_spanning_wavs();
    browser.wav_analyses[1].1 = browser.wav_analyses[1].1.into_one_shot();
    browser.wav_analyses[1].1.duration_seconds = 5.0;
    let short_loop = browser.wav_analyses[0].0.clone();
    let one_shot = browser.wav_analyses[1].0.clone();
    browser.wav_analyses[0].1.measures = 1;
    browser.track_grid = vec![
        vec![Some(LoopTrackClip::explicit(one_shot, 1))],
        vec![Some(LoopTrackClip::explicit(short_loop, 1))],
    ];
    browser.normalize_track_grid();

    assert_eq!(browser.track_grid[0].len(), 4);
    // 1 小節ループ側は 4 小節ぶん繰り返して埋まる。
    assert!(browser.playback_grid()[1].iter().all(Option::is_some));
}

#[test]
fn a_one_shot_span_rounds_up_to_a_power_of_two_without_an_upper_limit() {
    let mut browser = browser_with_spanning_wavs();
    browser.wav_analyses[1].1 = browser.wav_analyses[1].1.into_one_shot();
    let one_shot = browser.wav_analyses[1].0.clone();

    // 1 小節 = 2 秒として、鳴り終わるまでに要する小節数を 2 の冪へ切り上げる。
    for (duration_seconds, expected) in [
        (0.5, 1),
        (2.0, 1),
        (2.1, 2),
        (4.0, 2),
        (5.0, 4),
        (8.0, 4),
        (8.1, 8),
        (17.0, 16),
        (64.0, 32),
    ] {
        browser.wav_analyses[1].1.duration_seconds = duration_seconds;
        assert_eq!(
            browser.span_for_wav_at(&one_shot, 2.0),
            Some(expected),
            "{duration_seconds}s"
        );
    }

    // 壊れた解析値でグリッドの軸を暴走させない。
    browser.wav_analyses[1].1.duration_seconds = f64::NAN;
    assert_eq!(browser.span_for_wav_at(&one_shot, 2.0), Some(1));
    browser.wav_analyses[1].1.duration_seconds = 10.0;
    assert_eq!(browser.span_for_wav_at(&one_shot, 0.0), Some(1));
}
