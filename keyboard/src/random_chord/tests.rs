use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use cmrt_realtime_play::PatchVoicing;
use cmrt_tui_core::patch_load::PatchLoadState;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

use super::*;
use crate::{
    KeyboardContext, KeyboardMmlInput, KeyboardNoteGuide, KeyboardState, KeyboardVoicingLookup,
    NotePlaybackMode,
};

mod cancellation;
mod editing;

struct NoVoicing;

impl KeyboardVoicingLookup for NoVoicing {
    fn cached_voicing(&self, _patch: &str) -> Option<PatchVoicing> {
        None
    }
}

fn context(load: &PatchLoadState) -> KeyboardContext<'_> {
    KeyboardContext {
        patch_dirs_configured: false,
        patch_load: load,
        voicing: &NoVoicing,
        catalog_notes: &[],
    }
}

fn screen() -> KeyboardScreen<'static> {
    let mut state = KeyboardState::default();
    state.replace_repeat_chords(
        cmrt_chord::note_progression("ceg").unwrap(),
        Instant::now(),
        false,
    );
    KeyboardScreen::new(
        None,
        state,
        KeyboardMmlInput::restored("ceg".to_string()),
        KeyboardNoteGuide::new(None),
    )
}

fn target(screen: &KeyboardScreen<'_>) -> Vec<Vec<u8>> {
    screen
        .state
        .repeat_chords()
        .iter()
        .map(|chord| chord.iter().map(|note| note.midi_note).collect())
        .collect()
}

fn toggle(screen: &mut KeyboardScreen<'_>, ctx: &KeyboardContext<'_>) {
    screen.handle_key(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::SHIFT), ctx);
}

#[test]
fn keyboard_key_generates_one_matching_progression_while_off_and_keeps_it_during_cc_ticks() {
    let load = PatchLoadState::Loading;
    let ctx = context(&load);
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let mut screen = screen().with_chord_progression_source(Arc::new(move || {
        observed.fetch_add(1, Ordering::Relaxed);
        ChordProgressionCatalog::from_json(r#"[{"degrees":"I-IV-V-I"}]"#).unwrap()
    }));
    assert_eq!(calls.load(Ordering::Relaxed), 0);

    toggle(&mut screen, &ctx);

    assert!(screen.random_chord_mode());
    assert_eq!(screen.state.note_playback_mode(), NotePlaybackMode::Off);
    let mml = screen.mml_input.last_confirmed().to_string();
    let chords = target(&screen);
    assert_eq!(cmrt_chord::note_progression(&mml).unwrap(), chords);
    assert_eq!(chords.len(), 4);
    let now = Instant::now();
    screen.state.toggle_cc_periodic(now);
    for tick in 1..=40 {
        let event = screen
            .state
            .poll_periodic_tick(now + Duration::from_millis(250 * tick))
            .unwrap();
        assert!(event.messages.iter().all(|message| message[0] == 0xB0));
        assert_eq!(screen.mml_input.last_confirmed(), mml);
        assert_eq!(target(&screen), chords);
    }
    assert_eq!(calls.load(Ordering::Relaxed), 1);

    screen.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE), &ctx);
    assert_eq!(screen.mml_input.value(), mml);
}

#[test]
fn random_mode_toggle_does_not_repick_on_repeat_release_or_disable() {
    let load = PatchLoadState::Loading;
    let ctx = context(&load);
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let mut screen = screen().with_chord_progression_source(Arc::new(move || {
        observed.fetch_add(1, Ordering::Relaxed);
        ChordProgressionCatalog::from_json(r#"[{"degrees":"I-IV"}]"#).unwrap()
    }));
    toggle(&mut screen, &ctx);
    let saved = screen.session_state();
    for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
        screen.handle_key(
            KeyEvent::new_with_kind(KeyCode::Char('I'), KeyModifiers::SHIFT, kind),
            &ctx,
        );
        assert!(screen.random_chord_mode());
    }
    toggle(&mut screen, &ctx);
    assert!(!screen.random_chord_mode());
    assert_eq!(screen.session_state(), saved);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[test]
fn catalog_failures_preserve_confirmed_input_and_target_and_show_the_reason() {
    let load = PatchLoadState::Loading;
    let ctx = context(&load);
    let unplayable = ChordProgressionCatalog::from_json(r#"[{"degrees":"nonsense"}]"#).unwrap();
    for catalog in [
        None,
        Some(ChordProgressionCatalog::default()),
        Some(unplayable),
    ] {
        let mut screen = screen();
        if let Some(catalog) = catalog {
            screen.set_chord_progression_source(Arc::new(move || catalog.clone()));
        }
        let saved = screen.session_state();
        toggle(&mut screen, &ctx);
        assert!(!screen.random_chord_mode());
        assert_eq!(screen.session_state(), saved);
        assert!(!screen.mml_input.is_active());
        let reason = screen.random_chord_error().unwrap().to_string();
        let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
        terminal
            .draw(|f| crate::ui::draw(&mut screen, &Default::default(), Instant::now(), f))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let text = (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer.cell((x, y)).unwrap().symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.replace(' ', "").contains(&reason.replace(' ', "")),
            "{text}"
        );
    }
}

#[test]
fn lookahead_keeps_confirmed_mml_and_target_until_the_same_boundary_deadline() {
    for mode in [NotePlaybackMode::Repeat, NotePlaybackMode::Arp] {
        let mut screen = screen();
        screen.random_chord.enabled = true;
        screen.random_chord.catalog =
            ChordProgressionCatalog::from_json(r#"[{"degrees":"I-IV"}]"#).unwrap();
        let origin = Instant::now();
        while screen.state.note_playback_mode() != mode {
            screen.state.cycle_note_playback(origin);
        }
        let old_target = target(&screen);
        let cycle_ticks = if mode == NotePlaybackMode::Repeat {
            old_target.len() as u64 * 8
        } else {
            old_target.iter().map(|chord| chord.len() as u64 * 2).sum()
        };
        let boundary = origin + Duration::from_millis(250 * cycle_ticks);
        for step in 1..=cycle_ticks {
            let deadline = origin + Duration::from_millis(250 * step);
            let sent_at = deadline - crate::periodic_timeline::LOOKAHEAD;
            screen.apply_random_chord_progression(sent_at);
            screen.poll_random_chord_tick(deadline).unwrap();
            assert_eq!(
                screen.random_chord.pending_mml.is_some(),
                step == cycle_ticks
            );
            assert_eq!(screen.mml_input.last_confirmed(), "ceg");
            assert_eq!(target(&screen), old_target);
        }
        let next_mml = screen.random_chord.pending_mml.as_ref().unwrap().1.clone();
        screen.apply_random_chord_progression(boundary - Duration::from_millis(1));
        assert_eq!(screen.mml_input.last_confirmed(), "ceg");
        assert_eq!(target(&screen), old_target);
        screen.apply_random_chord_progression(boundary);
        assert_eq!(screen.mml_input.last_confirmed(), next_mml);
        assert_eq!(
            target(&screen),
            cmrt_chord::note_progression(&next_mml).unwrap()
        );
        assert!(screen.random_chord.pending_mml.is_none());
        assert_eq!(
            screen
                .state
                .sounding_position(boundary)
                .unwrap()
                .chord_index,
            0
        );
    }
}

#[test]
fn a_failed_cycle_pick_preserves_mml_and_target_and_waits_for_the_next_boundary() {
    let mut screen = screen();
    let origin = Instant::now();
    screen.state.cycle_note_playback(origin);
    screen.random_chord.enabled = true;
    let old_target = target(&screen);
    let boundary_step = old_target.len() as u64 * 8;
    for step in 1..boundary_step + 8 {
        let tick = screen
            .poll_random_chord_tick(origin + Duration::from_millis(250 * step))
            .unwrap();
        assert_eq!(screen.random_chord_error().is_some(), step >= boundary_step);
        assert_eq!(target(&screen), old_target);
        assert_eq!(screen.mml_input.last_confirmed(), "ceg");
        assert!(screen.random_chord.pending_mml.is_none());
        if step == boundary_step {
            assert!(tick.messages.contains(&[0x90, 60, 100]));
        }
    }
}
