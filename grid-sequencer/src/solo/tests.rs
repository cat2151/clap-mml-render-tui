use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::*;
use crate::{ChordPlayback, GridPatchLoad, GridSequencerContext, NoVoicingLookup};

const AREA: Rect = Rect::new(0, 0, 100, 24);

fn context() -> GridSequencerContext<'static> {
    crate::tests::ctx_with(
        GridPatchLoad::Ready(&[]),
        crate::tests::empty_catalog(),
        &NoVoicingLookup,
    )
}

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

#[test]
fn keyboard_navigation_selects_logical_tracks_and_clamps_at_the_edges() {
    let mut screen = GridSequencerScreen::with_track_count(None, 3);
    let now = Instant::now();

    screen.handle_key(press(KeyCode::Char('j')), now, &context());
    assert_eq!(screen.selected_track(), 1);
    screen.handle_key(press(KeyCode::Down), now, &context());
    assert_eq!(screen.selected_track(), 2);
    screen.handle_key(press(KeyCode::Char('j')), now, &context());
    assert_eq!(screen.selected_track(), 2);

    screen.handle_key(press(KeyCode::Char('k')), now, &context());
    assert_eq!(screen.selected_track(), 1);
    screen.handle_key(press(KeyCode::Up), now, &context());
    screen.handle_key(press(KeyCode::Up), now, &context());
    assert_eq!(screen.selected_track(), 0);
}

#[test]
fn s_supports_multiple_solos_and_the_last_toggle_restores_every_track() {
    let mut screen = GridSequencerScreen::with_track_count(None, 3);
    let now = Instant::now();

    screen.handle_key(press(KeyCode::Char('s')), now, &context());
    assert!(screen.track_is_soloed(0));
    assert!(screen.track_is_audible(0));
    assert!(!screen.track_is_audible(1));

    screen.handle_key(press(KeyCode::Down), now, &context());
    screen.handle_key(press(KeyCode::Char('s')), now, &context());
    assert!(screen.track_is_soloed(0));
    assert!(screen.track_is_soloed(1));

    screen.handle_key(press(KeyCode::Char('s')), now, &context());
    screen.handle_key(press(KeyCode::Up), now, &context());
    screen.handle_key(press(KeyCode::Char('s')), now, &context());
    assert!(!screen.solo_mode_active());
    assert!((0..3).all(|track| screen.track_is_audible(track)));
}

#[test]
fn clicking_the_s_column_selects_and_toggles_that_track() {
    let mut screen = GridSequencerScreen::with_track_count(None, 3);
    let layout = crate::ui::layout_for(&screen, AREA);

    screen.handle_mouse(
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            layout.solo_column(),
            layout.note.y + 3,
        ),
        AREA,
        &context(),
    );

    assert_eq!(screen.selected_track(), 1);
    assert!(screen.track_is_soloed(1));
}

#[test]
fn dragging_across_tracks_keeps_the_mouse_down_track_selected() {
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    let layout = crate::ui::layout_for(&screen, AREA);

    screen.handle_mouse(
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            layout.step_column(0),
            layout.note.y + 3,
        ),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(
            MouseEventKind::Drag(MouseButton::Left),
            layout.step_column(2),
            layout.note.y + 2,
        ),
        AREA,
        &context(),
    );

    assert_eq!(screen.selected_track(), 1);
}

#[test]
fn solo_mask_and_chord_boost_are_composed_for_both_banks() {
    let mut screen = GridSequencerScreen::with_track_count(None, 3);
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );
    screen.solo_tracks[CHORD_ROW] = true;

    let gains = screen.playback_amplitude_gains();
    let boosted = 10.0f32.powf(CHORD_GAIN_DB / 20.0);
    assert_eq!(gains.len(), 6);
    for bank in 0..cmrt_realtime_play::BANK_COUNT {
        let first = bank * 3;
        assert!((gains[first] - boosted).abs() < 0.0001);
        assert_eq!(gains[first + 1], 0.0);
        assert_eq!(gains[first + 2], 0.0);
    }
}

#[test]
fn track_count_changes_keep_surviving_solos_and_clamp_the_selection() {
    let mut screen = GridSequencerScreen::with_track_count(None, 3);
    screen.solo_tracks = vec![true, false, true];
    screen.selected_track = 2;

    screen.resize_for_restart(4, &[]);

    assert_eq!(screen.solo_tracks, vec![true, false, true, false]);
    assert_eq!(screen.selected_track(), 2);

    screen.resize_for_restart(2, &[]);

    assert_eq!(screen.solo_tracks, vec![true, false]);
    assert_eq!(screen.selected_track(), 1);
}

#[test]
fn leaving_and_resuming_the_screen_keeps_solo_and_track_selection() {
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.solo_tracks[1] = true;
    screen.selected_track = 1;

    screen.finish();

    assert!(screen.track_is_soloed(1));
    assert_eq!(screen.selected_track(), 1);
}

#[test]
fn restoring_an_app_session_clears_solo_and_selects_the_first_track() {
    let mut old = GridSequencerScreen::with_track_count(None, 2);
    old.solo_tracks[1] = true;
    old.selected_track = 1;
    old.grid_ready = true;
    let session = old.session_state().unwrap();

    let restored_after_app_restart = GridSequencerScreen::new_with(crate::GridSequencerParts {
        track_count: 2,
        restored_session: Some(session),
        ..crate::GridSequencerParts::default()
    });

    assert!(!restored_after_app_restart.solo_mode_active());
    assert_eq!(restored_after_app_restart.selected_track(), 0);
}
