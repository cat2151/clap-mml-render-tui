use super::*;
use crate::DawGridImportTrack;

fn song(measures: &[&str]) -> DawGridImportSong {
    DawGridImportSong {
        bpm: 120.0,
        chord: None,
        tracks: vec![DawGridImportTrack {
            patch: Some("Keys/Piano.fxp".to_string()),
            swing: 50,
            measures: measures.iter().map(|mml| (*mml).to_string()).collect(),
            chord_binding: None,
        }],
    }
}

#[test]
fn preview_prepares_only_the_first_measure_with_the_import_patch() {
    let prepared = prepare_first_measure(song(&["o5c4", "o6g4"]), 48_000.0).unwrap();

    assert_eq!(prepared.active_tracks, [crate::FIRST_PLAYABLE_TRACK]);
    let mml = &prepared.track_mmls[crate::FIRST_PLAYABLE_TRACK];
    assert!(mml.contains("Keys/Piano.fxp"), "{mml}");
    assert!(mml.contains("o5c4"), "{mml}");
    assert!(!mml.contains("o6g4"), "{mml}");
}

#[test]
fn preview_rejects_a_song_without_a_playable_first_measure() {
    let error = prepare_first_measure(song(&[""]), 48_000.0)
        .err()
        .expect("empty first measure should fail");

    assert!(error.to_string().contains("1小節目"));
}

#[test]
fn preview_cache_key_ignores_later_measures_but_includes_the_first() {
    let first = prepare_first_measure(song(&["o5c4", "o6c4"]), 48_000.0).unwrap();
    let later_changed = prepare_first_measure(song(&["o5c4", "o2d4"]), 48_000.0).unwrap();
    let first_changed = prepare_first_measure(song(&["o5d4", "o6c4"]), 48_000.0).unwrap();

    assert_eq!(first.key, later_changed.key);
    assert_ne!(first.key, first_changed.key);
}

#[test]
fn silent_tracks_are_not_sent_to_the_offline_renderer() {
    let mut song = song(&["o5c4"]);
    song.tracks.push(DawGridImportTrack {
        patch: Some("Pads/Silent.fxp".to_string()),
        swing: 50,
        measures: vec!["r1".to_string()],
        chord_binding: None,
    });

    let prepared = prepare_first_measure(song, 48_000.0).unwrap();

    assert_eq!(prepared.active_tracks, [crate::FIRST_PLAYABLE_TRACK]);
}

#[test]
fn rapid_preview_navigation_keeps_only_the_latest_pending_song() {
    let player =
        DawGridPreviewPlayer::disabled_for_tests(Arc::new(cmrt_runtime::Config::default()));
    *player.runtime.in_flight.lock().unwrap() = Some((u64::MAX, 0));
    let first = prepare_first_measure(song(&["o5c4"]), 48_000.0).unwrap();
    let latest = prepare_first_measure(song(&["o5d4"]), 48_000.0).unwrap();

    player.ensure_render(first.clone(), RenderPriority::High);
    player.ensure_render(latest.clone(), RenderPriority::High);

    let pending_key = player
        .runtime
        .pending
        .lock()
        .unwrap()
        .as_ref()
        .map(|(prepared, _, _)| prepared.key);
    assert_eq!(pending_key, Some(latest.key));

    *player.runtime.in_flight.lock().unwrap() = Some((first.key, 0));
    player.ensure_render(first, RenderPriority::High);
    assert!(player.runtime.pending.lock().unwrap().is_none());
    player.stop();
    assert!(player.runtime.pending.lock().unwrap().is_none());
}

#[test]
fn a_new_preview_after_stop_inherits_the_finishing_render_slot() {
    let player =
        DawGridPreviewPlayer::disabled_for_tests(Arc::new(cmrt_runtime::Config::default()));
    *player.runtime.in_flight.lock().unwrap() = Some((u64::MAX, 0));
    player.stop();
    let latest = prepare_first_measure(song(&["o5e4"]), 48_000.0).unwrap();

    player.ensure_render(latest.clone(), RenderPriority::High);
    let next = claim_pending_render(&player.runtime).expect("latest preview should be handed off");

    assert_eq!(next.0.key, latest.key);
    assert_eq!(
        *player.runtime.in_flight.lock().unwrap(),
        Some((latest.key, 1))
    );
}

#[test]
fn same_key_from_a_new_generation_waits_for_and_reuses_the_in_flight_render() {
    let player =
        DawGridPreviewPlayer::disabled_for_tests(Arc::new(cmrt_runtime::Config::default()));
    let prepared = prepare_first_measure(song(&["o5g4"]), 48_000.0).unwrap();
    *player.runtime.in_flight.lock().unwrap() = Some((prepared.key, 0));
    player.runtime.render_generation.store(1, Ordering::Release);

    player.ensure_render(prepared.clone(), RenderPriority::High);
    assert_eq!(
        player
            .runtime
            .pending
            .lock()
            .unwrap()
            .as_ref()
            .map(|(_, _, generation)| *generation),
        Some(1)
    );

    player
        .runtime
        .cache
        .lock()
        .unwrap()
        .insert(prepared.key, Arc::new(vec![0.0]));
    finish_render_slot(player.runtime.clone());
    assert!(player.runtime.pending.lock().unwrap().is_none());
    assert!(player.runtime.in_flight.lock().unwrap().is_none());
}

#[test]
fn stale_preparation_cannot_replace_the_latest_preview_request() {
    let output = PreviewOutput::new(48_000);
    let stale = output.begin_preparing(1);
    let latest = output.begin_preparing(7);

    assert!(!output.finish_preparing(stale, 10, 1));
    output.fail_to_prepare(stale, "stale error".to_string());
    assert_eq!(
        output.status(),
        DawGridPreviewStatus::Rendering {
            completed: 0,
            total: 7,
        }
    );

    assert!(output.finish_preparing(latest, 20, 5));
    assert_eq!(
        output.status(),
        DawGridPreviewStatus::Rendering {
            completed: 0,
            total: 5,
        }
    );
}
