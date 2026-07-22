use super::*;
use crate::loop_browser::random::{load_from as load_random_decks, LoopRandomScope};

fn temp_random_path(name: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!(
            "cmrt-batch-random-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .join("loop_browser")
        .join("random_decks.toml")
}

fn enable_random_persistence(browser: &mut LoopBrowser, name: &str) -> PathBuf {
    let path = temp_random_path(name);
    browser.random_decks.path = Some(path.clone());
    path
}

fn set_bpm(browser: &mut LoopBrowser, index: usize, bpm: f64) {
    let tempo = browser.wav_analyses[index].1.tempo.as_mut().unwrap();
    tempo.bpm = bpm;
    tempo.declared_bpm = Some(bpm);
}

fn has_bpm_red(browser: &LoopBrowser) -> bool {
    let target = browser.target_bpm().bpm;
    browser.track_grid.iter().flatten().any(|cell| {
        cell.as_ref()
            .is_some_and(|clip| browser.clip_exceeds_time_ratio_limits(clip, target))
    })
}

#[test]
fn shift_r_randomizes_every_track_at_the_current_measure_without_bpm_red() {
    let mut browser = browser_with_direct_wavs(2);
    set_bpm(&mut browser, 0, 60.0);
    set_bpm(&mut browser, 1, 120.0);
    let slow = browser.wav_analyses[0].0.clone();
    browser.track_grid = vec![
        vec![Some(LoopTrackClip::explicit(slow.clone(), 1))],
        vec![Some(LoopTrackClip::explicit(slow, 1))],
    ];
    browser.focus = LoopBrowserPane::Tracks;
    let path = enable_random_persistence(&mut browser, "all-tracks");

    let action = browser.handle_key_event(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT));

    assert!(matches!(
        action,
        LoopBrowserAction::GridReplaced {
            start_measure: 0,
            reason: LoopGridChange::BatchRandom,
            ..
        }
    ));
    assert!(browser.track_grid.iter().all(|track| track[0]
        .as_ref()
        .is_some_and(|clip| clip.wav.relative == "01.wav")));
    assert!(!has_bpm_red(&browser));
    assert!(load_random_decks(&path).is_ok());
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}

#[test]
fn shift_r_inserts_into_empty_tracks_and_keeps_other_measures() {
    let mut browser = browser_with_direct_wavs(2);
    let first = browser.wav_analyses[0].0.clone();
    let second = browser.wav_analyses[1].0.clone();
    browser.track_grid = vec![
        vec![
            Some(LoopTrackClip::explicit(first, 1)),
            Some(LoopTrackClip::explicit(second.clone(), 1)),
        ],
        vec![None, Some(LoopTrackClip::explicit(second.clone(), 1))],
    ];
    browser.focus = LoopBrowserPane::Tracks;
    let path = enable_random_persistence(&mut browser, "empty-track");

    browser.handle_key_event(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT));

    assert!(browser.track_grid[0][0].is_some());
    assert!(browser.track_grid[1][0].is_some());
    assert!(browser.track_grid[0][1]
        .as_ref()
        .is_some_and(|clip| clip.wav.matches(&second)));
    assert!(browser.track_grid[1][1]
        .as_ref()
        .is_some_and(|clip| clip.wav.matches(&second)));
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}

#[test]
fn shift_r_on_a_continuation_replaces_its_explicit_clip_and_clears_the_old_span() {
    let mut browser = browser_with_spanning_wavs();
    let long = browser.wav_analyses[0].0.clone();
    browser.track_grid = vec![vec![Some(LoopTrackClip::explicit(long, 2)), None, None]];
    browser.normalize_track_grid();
    browser.focus = LoopBrowserPane::Tracks;
    browser.measure_cursor = 1;
    let path = enable_random_persistence(&mut browser, "continuation");

    let action = browser.handle_key_event(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT));

    assert!(matches!(
        action,
        LoopBrowserAction::GridReplaced {
            start_measure: 0,
            ..
        }
    ));
    assert!(browser.track_grid[0][0].as_ref().is_some_and(|clip| {
        clip.wav.relative == "short.wav" && clip.span_measures == 1 && !clip.is_previous()
    }));
    assert!(browser.clip_at(0, 1).is_none());
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}

#[test]
fn incompatible_category_candidates_roll_back_the_whole_batch() {
    let mut browser = browser_with_direct_wavs(2);
    set_bpm(&mut browser, 0, 60.0);
    set_bpm(&mut browser, 1, 200.0);
    let slow = browser.wav_analyses[0].0.clone();
    let fast = browser.wav_analyses[1].0.clone();
    browser
        .wav_categories
        .insert(slow.lookup_key(), "slow".to_string());
    browser
        .wav_categories
        .insert(fast.lookup_key(), "fast".to_string());
    browser.track_grid = vec![
        vec![Some(LoopTrackClip::explicit(slow, 1))],
        vec![Some(LoopTrackClip::explicit(fast, 1))],
    ];
    browser.focus = LoopBrowserPane::Tracks;
    let original_grid = browser.track_grid.clone();
    let original_decks = browser.random_decks.value.clone();
    let path = enable_random_persistence(&mut browser, "incompatible");

    let action = browser.handle_key_event(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT));

    assert!(matches!(action, LoopBrowserAction::Continue));
    assert_eq!(browser.track_grid, original_grid);
    assert_eq!(browser.random_decks.value, original_decks);
    assert!(browser
        .active_notice()
        .is_some_and(|notice| notice.text.contains("解消できる組み合わせがありません")));
    assert!(!path.exists());
}

#[test]
fn track_grid_save_failure_restores_grid_and_random_deck() {
    let mut browser = browser_with_direct_wavs(2);
    let original = browser.wav_analyses[0].0.clone();
    browser.track_grid[0][0] = Some(LoopTrackClip::explicit(original, 1));
    browser.focus = LoopBrowserPane::Tracks;
    let original_grid = browser.track_grid.clone();
    let original_decks = browser.random_decks.value.clone();
    let random_path = enable_random_persistence(&mut browser, "grid-failure");
    let blocked = random_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("blocked-grid");
    std::fs::create_dir_all(&blocked).unwrap();
    browser.track_grid_path = Some(blocked.clone());

    let action = browser.handle_key_event(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT));

    assert!(matches!(action, LoopBrowserAction::Continue));
    assert_eq!(browser.track_grid, original_grid);
    assert_eq!(browser.random_decks.value, original_decks);
    assert_eq!(load_random_decks(&random_path).unwrap(), original_decks);
    assert!(browser
        .track_grid_error
        .as_deref()
        .is_some_and(|error| error.contains("track listを保存できません")));
    let _ = std::fs::remove_dir_all(random_path.parent().unwrap().parent().unwrap());
}

#[test]
fn batch_random_uses_the_original_category_for_each_track() {
    let mut browser = browser_with_direct_wavs(4);
    let slow_a = browser.wav_analyses[0].0.clone();
    let slow_b = browser.wav_analyses[1].0.clone();
    let fast_a = browser.wav_analyses[2].0.clone();
    let fast_b = browser.wav_analyses[3].0.clone();
    for wav in [&slow_a, &slow_b] {
        browser
            .wav_categories
            .insert(wav.lookup_key(), "slow".to_string());
    }
    for wav in [&fast_a, &fast_b] {
        browser
            .wav_categories
            .insert(wav.lookup_key(), "fast".to_string());
    }
    browser.track_grid = vec![
        vec![Some(LoopTrackClip::explicit(slow_a, 1))],
        vec![Some(LoopTrackClip::explicit(fast_a, 1))],
    ];
    browser.focus = LoopBrowserPane::Tracks;
    let path = enable_random_persistence(&mut browser, "categories");

    browser.handle_key_event(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT));

    assert_eq!(
        browser.category_for_wav(&browser.track_grid[0][0].as_ref().unwrap().wav),
        Some("slow")
    );
    assert_eq!(
        browser.category_for_wav(&browser.track_grid[1][0].as_ref().unwrap().wav),
        Some("fast")
    );
    assert_eq!(
        browser
            .random_candidates(&LoopRandomScope::Category {
                category: "slow".to_string()
            })
            .len(),
        2
    );
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}
