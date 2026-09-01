use super::*;
use crate::{ChordPlayback, GridInstance, NotePattern};

fn snapshot_with(pattern: NotePattern) -> GridSongSnapshot {
    let mut instance = GridInstance::new(0);
    instance.patch = Some("Keys/Piano.fxp".to_string());
    instance.lanes[0].base_note = 60;
    instance.lanes[0].pattern = pattern;
    instance.swing = 62;
    GridSongSnapshot::new(123.0, vec![instance], None)
}

#[test]
fn a_grid_pattern_becomes_one_measure_of_mml() {
    let mut pattern = NotePattern::default();
    pattern.draw_span(0, 3);
    pattern.draw_span(8, 9);

    let tracks = snapshot_with(pattern).daw_tracks();

    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].patch.as_deref(), Some("Keys/Piano.fxp"));
    assert_eq!(tracks[0].swing, 62);
    assert_eq!(tracks[0].measures, ["o5c4r4o5c8r4."]);
}

#[test]
fn chord_mode_expands_each_chord_to_a_daily_daw_measure() {
    let mut instances = vec![GridInstance::new(0), GridInstance::new(1)];
    instances[0].patch = Some("Pads/Poly.fxp".to_string());
    let chord = ChordPlayback::new(
        "C",
        "I-V".to_string(),
        vec![vec![60, 64, 67], vec![67, 71, 74]],
    )
    .unwrap();

    let snapshot = GridSongSnapshot::new(120.0, instances, Some(chord));
    let tracks = snapshot.daw_tracks();

    assert_eq!(snapshot.measure_count(), 2);
    assert_eq!(tracks[0].measures[0], "o5c1;o5e1;o5g1");
    assert_eq!(tracks[0].measures[1], "o5g1;o5b1;o6d1");
    assert!(matches!(
        tracks[0].chord_binding,
        Some(GridDawChordBinding::Chord)
    ));
    assert!(matches!(
        tracks[1].chord_binding,
        Some(GridDawChordBinding::Bass { .. })
    ));
    let source = snapshot.daw_chord_source().expect("semantic chord source");
    assert_eq!(source.init, "key:C");
    assert_eq!(source.measures, ["I", "V"]);
    assert_eq!(source.voicings[0].notes, [60, 64, 67]);
}

#[test]
fn newest_history_is_selected_and_enter_returns_it() {
    let mut screen = GridSequencerScreen::new(None);
    screen.history.push(snapshot_with(NotePattern::default()));
    let mut latest = NotePattern::default();
    latest.draw_span(0, 0);
    screen.history.push(snapshot_with(latest));
    screen.history.open(false);

    let action = screen.handle_history_key(
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        Instant::now(),
    );

    let GridSequencerAction::ImportToDailyDaw(snapshot) = action else {
        panic!("expected Daily DAW import");
    };
    assert_eq!(snapshot.daw_tracks()[0].measures, ["o5c16r2..."]);
    assert!(!screen.history_open());
}

#[test]
fn consecutive_identical_snapshots_are_recorded_once() {
    let mut history = GridHistory::default();
    let snapshot = snapshot_with(NotePattern::default());

    history.push(snapshot.clone());
    history.push(snapshot);

    assert_eq!(history.entries.len(), 1);
}

#[test]
fn history_navigation_moves_from_newer_to_older() {
    let mut history = GridHistory::default();
    history.push(snapshot_with(NotePattern::default()));
    let mut latest = NotePattern::default();
    latest.draw_span(0, 0);
    history.push(snapshot_with(latest));
    history.open(false);

    assert!(history.next_older());
    assert!(!history.next_older());
    assert_eq!(history.selected, 1);
    assert!(history.next_newer());
    assert_eq!(history.selected, 0);
}

#[test]
fn navigation_at_the_history_edge_does_not_restart_the_same_preview() {
    let mut screen = GridSequencerScreen::new(None);
    screen.history.push(snapshot_with(NotePattern::default()));
    screen.history.open(false);

    let action = screen.handle_history_key(
        KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
        Instant::now(),
    );

    assert!(matches!(action, GridSequencerAction::Continue));
}

#[test]
fn moving_to_another_history_automatically_starts_its_preview() {
    let mut screen = GridSequencerScreen::new(None);
    screen.history.push(snapshot_with(NotePattern::default()));
    let mut latest = NotePattern::default();
    latest.draw_span(0, 0);
    screen.history.push(snapshot_with(latest));
    screen.history.open(false);

    let action = screen.handle_history_key(
        KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
        Instant::now(),
    );

    assert!(matches!(
        action,
        GridSequencerAction::PlayDailyDawPreview(_)
    ));
    assert!(screen.history_previewing());
}
