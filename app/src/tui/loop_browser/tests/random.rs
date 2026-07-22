use super::*;
use crate::loop_browser::random::{load_from as load_random_decks, LoopRandomScope};
use std::collections::HashSet;
use std::time::{Duration, Instant};

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!(
            "cmrt-loop-random-{name}-{}-{}",
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
    let path = temp_path(name);
    browser.random_decks.path = Some(path.clone());
    path
}

fn preview_path(action: LoopBrowserAction) -> PathBuf {
    match action {
        LoopBrowserAction::Preview(path) => path,
        _ => panic!("random selection should preview a WAV"),
    }
}

#[test]
fn tree_random_draws_all_wavs_without_repeats_and_reveals_each_target() {
    let mut browser = browser();
    let path = enable_random_persistence(&mut browser, "tree-all");
    let mut selected = HashSet::new();

    for _ in 0..3 {
        let preview = preview_path(browser.handle_key(KeyCode::Char('r')));
        assert!(selected.insert(preview.clone()));
        assert!(browser.visible[browser.cursor].is_wav);
        assert_eq!(browser.visible[browser.cursor].path, preview);
    }

    assert_eq!(selected.len(), 3);
    assert!(path.exists());
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}

#[test]
fn zero_candidates_do_nothing_and_one_candidate_can_repeat() {
    let mut empty = LoopBrowser::default();
    let empty_path = enable_random_persistence(&mut empty, "empty-library");
    assert!(matches!(
        empty.handle_key(KeyCode::Char('r')),
        LoopBrowserAction::Continue
    ));
    assert!(!empty_path.exists());

    let mut single = browser_with_direct_wavs(1);
    let single_path = enable_random_persistence(&mut single, "single-wav");
    let first = preview_path(single.handle_key(KeyCode::Char('r')));
    let second = preview_path(single.handle_key(KeyCode::Char('r')));
    assert_eq!(first, second);
    let _ = std::fs::remove_dir_all(single_path.parent().unwrap().parent().unwrap());
}

#[test]
fn favorites_only_random_draws_only_favorite_wavs_without_nested_duplicates() {
    let mut browser = browser();
    let path = enable_random_persistence(&mut browser, "favorites");
    browser
        .metadata
        .value
        .toggle_favorite(&LoopDirId::new(Path::new("/loops"), Path::new("Pack")));
    browser
        .metadata
        .value
        .toggle_favorite(&LoopDirId::new(Path::new("/loops"), Path::new("Pack/Bass")));
    browser.rebuild_favorite_wav_keys();
    browser.rebuild_visible(None);
    browser.handle_key(KeyCode::Char('V'));
    let mut selected = HashSet::new();

    for _ in 0..3 {
        selected.insert(preview_path(browser.handle_key(KeyCode::Char('r'))));
    }

    assert_eq!(selected.len(), 3);
    assert!(selected.iter().all(|wav| wav.starts_with("/loops/Pack")));
    assert!(browser.favorites_only);
    assert!(browser.visible[browser.cursor].is_wav);
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}

#[test]
fn track_random_uses_category_for_a_continuation_cell() {
    let mut browser = browser();
    let path = enable_random_persistence(&mut browser, "track-category-continuation");
    browser.metadata.value.toggle_category(
        &LoopDirId::new(Path::new("/loops"), Path::new("Pack/Bass")),
        "bass",
    );
    browser.rebuild_wav_categories();
    let bass_wav = browser
        .wav_analyses
        .iter()
        .find(|(wav, _)| wav.relative.ends_with("Bass/a.wav"))
        .unwrap()
        .0
        .clone();
    browser.track_grid[0] = vec![Some(LoopTrackClip::explicit(bass_wav.clone(), 2)), None];
    browser
        .track_grid
        .push(vec![Some(LoopTrackClip::explicit(bass_wav, 2)), None]);
    browser.focus = LoopBrowserPane::Tracks;
    browser.measure_cursor = 1;

    let action = browser.handle_key(KeyCode::Char('r'));

    assert!(matches!(
        action,
        LoopBrowserAction::GridReplaced { start_measure: 0, grid, .. }
            if grid[0][0].as_ref().is_some_and(|clip| clip.path.ends_with("Pack/Bass/B.wav"))
                && grid[0][1]
                    .as_ref()
                    .is_some_and(|clip| clip.path.ends_with("Pack/Bass/B.wav"))
    ));
    assert!(browser.track_grid[0][0].as_ref().is_some_and(|clip| clip
        .wav
        .relative
        .ends_with("Pack/Bass/B.wav")
        && clip.span_measures == 1));
    assert!(browser
        .clip_at(0, 1)
        .is_some_and(|(_, clip)| clip.is_previous()));
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}

#[test]
fn track_random_draws_from_the_same_category_across_favorite_boundaries() {
    let mut browser = browser();
    let path = enable_random_persistence(&mut browser, "track-category-global");
    let bass_dir = LoopDirId::new(Path::new("/loops"), Path::new("Pack/Bass"));
    let drums_dir = LoopDirId::new(Path::new("/loops"), Path::new("Pack/Drums"));
    let spoken_dir = LoopDirId::new(Path::new("/loops"), Path::new("Spoken"));
    let spoken_wav = LoopWavId::new(Path::new("/loops"), Path::new("Spoken/Voice.wav"));
    let analysis = browser.wav_analyses[0].1;
    browser.wav_analyses.push((spoken_wav.clone(), analysis));
    browser.metadata.value.toggle_favorite(&bass_dir);
    browser.metadata.value.toggle_category(&bass_dir, "drum");
    browser.metadata.value.toggle_category(&drums_dir, "drum");
    browser
        .metadata
        .value
        .toggle_category(&spoken_dir, "spoken");
    browser.rebuild_favorite_wav_keys();
    browser.rebuild_wav_categories();
    let current = browser
        .wav_analyses
        .iter()
        .find(|(wav, _)| wav.relative.ends_with("Bass/a.wav"))
        .unwrap()
        .0
        .clone();
    browser.track_grid[0][0] = Some(LoopTrackClip::explicit(current, 1));
    browser.focus = LoopBrowserPane::Tracks;
    let mut selected = HashSet::new();

    for _ in 0..3 {
        assert!(matches!(
            browser.handle_key(KeyCode::Char('r')),
            LoopBrowserAction::GridReplaced { .. }
        ));
        selected.insert(
            browser.track_grid[0][0]
                .as_ref()
                .unwrap()
                .wav
                .relative
                .clone(),
        );
    }

    assert_eq!(selected.len(), 3);
    assert!(selected
        .iter()
        .all(|wav| wav.starts_with("Pack/Bass/") || wav == "Pack/Drums/Kick.wav"));
    assert!(selected.contains("Pack/Drums/Kick.wav"));
    assert!(!selected.contains(&spoken_wav.relative));
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}

#[test]
fn empty_track_cell_assigns_an_all_scope_wav_and_refreshes_without_preview() {
    let mut browser = browser();
    let path = enable_random_persistence(&mut browser, "empty-track");
    browser.favorites_only = true;
    browser.rebuild_visible(None);
    browser.focus = LoopBrowserPane::Tracks;

    let action = browser.handle_key(KeyCode::Char('r'));
    let selected = browser.track_grid[0][0]
        .as_ref()
        .expect("random WAV should be assigned to the empty cell")
        .wav
        .path();

    assert!(matches!(
        action,
        LoopBrowserAction::GridRefresh { grid, .. } if grid[0][0]
            .as_ref()
            .is_some_and(|clip| clip.path == selected)
    ));
    assert!(!browser.favorites_only);
    assert_eq!(browser.visible[browser.cursor].path, selected);
    assert!(browser.visible[browser.cursor].is_wav);
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}

#[test]
fn empty_track_random_and_pad_assignment_use_the_same_playback_grid_path() {
    let mut random_browser = browser_with_direct_wavs(1);
    random_browser.wav_analyses[0].1.tempo.as_mut().unwrap().bpm = 160.0;
    let random_path = enable_random_persistence(&mut random_browser, "empty-track-parity");
    random_browser.focus = LoopBrowserPane::Tracks;

    let mut pad_browser = browser_with_direct_wavs(1);
    pad_browser.wav_analyses[0].1.tempo.as_mut().unwrap().bpm = 160.0;
    let wav = pad_browser.wav_analyses[0].0.clone();
    pad_browser.metadata.value.toggle_pad('c', &wav);
    pad_browser.focus = LoopBrowserPane::Tracks;

    let random_grid = match random_browser.handle_key(KeyCode::Char('r')) {
        LoopBrowserAction::GridRefresh {
            grid,
            reason: LoopGridChange::Random,
        } => grid,
        _ => panic!("empty-cell random assignment should refresh the grid"),
    };
    let pad_grid = match pad_browser.handle_key(KeyCode::Char('c')) {
        LoopBrowserAction::GridRefresh {
            grid,
            reason: LoopGridChange::Pad('c'),
        } => grid,
        _ => panic!("empty-cell pad assignment should refresh the grid"),
    };

    assert_eq!(random_grid, pad_grid);
    assert_eq!(random_grid[0][0].as_ref().unwrap().bpm, Some(160.0));
    let _ = std::fs::remove_dir_all(random_path.parent().unwrap().parent().unwrap());
}

#[test]
fn empty_track_cell_save_failure_does_not_fall_back_to_preview() {
    let mut browser = browser_with_direct_wavs(2);
    let random_path = enable_random_persistence(&mut browser, "empty-grid-failure-random");
    let blocked = temp_path("empty-grid-failure-grid");
    std::fs::create_dir_all(&blocked).unwrap();
    browser.track_grid_path = Some(blocked.clone());
    browser.focus = LoopBrowserPane::Tracks;
    let original = browser.track_grid.clone();

    let action = browser.handle_key(KeyCode::Char('r'));

    assert!(matches!(action, LoopBrowserAction::Continue));
    assert_eq!(browser.track_grid, original);
    assert!(browser
        .track_grid_error
        .as_deref()
        .is_some_and(|error| error.contains("track listを保存できません")));
    let _ = std::fs::remove_dir_all(random_path.parent().unwrap().parent().unwrap());
    let _ = std::fs::remove_dir_all(blocked.parent().unwrap().parent().unwrap());
}

#[test]
fn nonfavorite_track_cell_uses_the_all_wav_deck() {
    let mut browser = browser();
    let path = enable_random_persistence(&mut browser, "nonfavorite-track");
    browser
        .metadata
        .value
        .toggle_favorite(&LoopDirId::new(Path::new("/loops"), Path::new("Pack/Bass")));
    browser.rebuild_favorite_wav_keys();
    let kick = browser
        .wav_analyses
        .iter()
        .find(|(wav, _)| wav.relative.ends_with("Drums/Kick.wav"))
        .unwrap()
        .0
        .clone();
    browser.track_grid[0][0] = Some(LoopTrackClip::explicit(kick, 1));
    browser.focus = LoopBrowserPane::Tracks;

    let action = browser.handle_key(KeyCode::Char('r'));
    let selected = browser.track_grid[0][0].as_ref().unwrap().wav.path();

    assert!(matches!(
        action,
        LoopBrowserAction::GridReplaced {
            start_measure: 0,
            ..
        }
    ));
    assert!(!selected.ends_with("Drums/Kick.wav"));
    assert!(browser.track_grid[0][0]
        .as_ref()
        .is_some_and(|clip| clip.wav.path() == selected));
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}

#[test]
fn track_random_replacement_grows_span_and_removes_overlapping_clip() {
    let mut browser = browser_with_spanning_wavs();
    let long = browser.wav_analyses[0].0.clone();
    let short = browser.wav_analyses[1].0.clone();
    browser.track_grid[0] = vec![
        Some(LoopTrackClip::explicit(short.clone(), 1)),
        Some(LoopTrackClip::explicit(short, 1)),
    ];
    browser.focus = LoopBrowserPane::Tracks;

    assert_eq!(browser.replace_current_clip(long), Some(0));

    assert_eq!(browser.track_grid[0][0].as_ref().unwrap().span_measures, 2);
    assert!(browser.track_grid[0][1].is_none());
}

#[test]
fn empty_track_insertion_grows_span_and_removes_overlapping_clip() {
    let mut browser = browser_with_spanning_wavs();
    let long = browser.wav_analyses[0].0.clone();
    let short = browser.wav_analyses[1].0.clone();
    browser.track_grid[0] = vec![None, Some(LoopTrackClip::explicit(short, 1))];
    browser.focus = LoopBrowserPane::Tracks;

    assert_eq!(browser.insert_current_clip(long), Some(0));

    assert_eq!(browser.track_grid[0][0].as_ref().unwrap().span_measures, 2);
    assert!(browser.track_grid[0][1].is_none());
}

#[test]
fn track_grid_save_failure_keeps_the_old_clip_and_only_previews() {
    let mut browser = browser_with_direct_wavs(2);
    let random_path = enable_random_persistence(&mut browser, "track-grid-failure-random");
    let blocked = temp_path("track-grid-failure-grid");
    std::fs::create_dir_all(&blocked).unwrap();
    browser.track_grid_path = Some(blocked.clone());
    let original = browser.wav_analyses[0].0.clone();
    browser.track_grid[0][0] = Some(LoopTrackClip::explicit(original.clone(), 1));
    browser.focus = LoopBrowserPane::Tracks;

    let action = browser.handle_key(KeyCode::Char('r'));

    assert!(
        matches!(action, LoopBrowserAction::Preview(path) if !path.ends_with(&original.relative))
    );
    assert!(browser.track_grid[0][0]
        .as_ref()
        .is_some_and(|clip| clip.wav.matches(&original)));
    assert!(browser
        .track_grid_error
        .as_deref()
        .is_some_and(|error| error.contains("track listを保存できません")));
    let _ = std::fs::remove_dir_all(random_path.parent().unwrap().parent().unwrap());
    let _ = std::fs::remove_dir_all(blocked.parent().unwrap().parent().unwrap());
}

#[test]
fn saved_browser_deck_continues_after_reconstruction() {
    let path = temp_path("restart");
    let mut first_browser = browser();
    first_browser.random_decks.path = Some(path.clone());
    let first = preview_path(first_browser.handle_key(KeyCode::Char('r')));

    let mut second_browser = browser();
    second_browser.random_decks.value = load_random_decks(&path).unwrap();
    second_browser.random_decks.path = Some(path.clone());
    let second = preview_path(second_browser.handle_key(KeyCode::Char('r')));

    assert_ne!(first, second);
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}

#[test]
fn save_failure_rolls_back_deck_and_does_not_move_or_preview() {
    let mut browser = browser();
    let blocked = temp_path("blocked");
    std::fs::create_dir_all(&blocked).unwrap();
    browser.random_decks.path = Some(blocked.clone());
    let cursor = browser.cursor;
    let state = browser.random_decks.value.clone();

    assert!(matches!(
        browser.handle_key(KeyCode::Char('r')),
        LoopBrowserAction::Continue
    ));

    assert_eq!(browser.cursor, cursor);
    assert_eq!(browser.random_decks.value, state);
    assert!(browser
        .random_decks
        .error
        .as_deref()
        .is_some_and(|error| error.contains("random deckを保存できません")));
    let _ = std::fs::remove_dir_all(blocked.parent().unwrap().parent().unwrap());
}

#[test]
fn all_scope_builds_realistic_candidate_list_without_quadratic_delay() {
    const WAV_COUNT: usize = 6_914;
    let mut browser = LoopBrowser::default();
    let analysis = indexed("unused.wav").analysis;
    browser.wav_analyses = (0..WAV_COUNT)
        .map(|index| {
            (
                LoopWavId::new(
                    Path::new("/loops"),
                    Path::new(&format!("library/{index:05}.wav")),
                ),
                analysis,
            )
        })
        .collect();
    browser.rebuild_wav_analysis_indices();

    let started_at = Instant::now();
    let candidates = browser.random_candidates(&LoopRandomScope::All);

    assert_eq!(candidates.len(), WAV_COUNT);
    assert!(
        started_at.elapsed() < Duration::from_secs(5),
        "6,914件の候補生成が線形時間の予算を超えました: {:?}",
        started_at.elapsed()
    );
}
