use std::time::Instant;

use cmrt_realtime_play::PatchVoicing;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::{
    tests::{ctx_with, empty_catalog},
    GridPatchLoad, GridSequencerAction, GridVoicingLookup,
};

struct PolyPatch;

impl GridVoicingLookup for PolyPatch {
    fn cached_voicing(&self, _patch: &str) -> Option<PatchVoicing> {
        Some(PatchVoicing::Poly)
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn type_text(
    screen: &mut GridSequencerScreen,
    text: &str,
    now: Instant,
    ctx: &GridSequencerContext<'_>,
) {
    for character in text.chars() {
        assert!(matches!(
            screen.handle_key(key(KeyCode::Char(character)), now, ctx),
            GridSequencerAction::Continue
        ));
    }
}

fn setup() -> (
    GridSequencerScreen,
    Vec<(String, String)>,
    &'static cmrt_chord::ChordProgressionCatalog,
) {
    (
        GridSequencerScreen::new(None),
        vec![("Keys/Poly.fxp".to_string(), "keys/poly.fxp".to_string())],
        empty_catalog(),
    )
}

#[test]
fn i_opens_a_single_line_input_and_enter_fixes_the_progression() {
    let now = Instant::now();
    let (mut screen, patches, catalog) = setup();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), catalog, &PolyPatch);
    screen.start(now, &ctx);

    screen.handle_key(key(KeyCode::Char('i')), now, &ctx);
    assert!(screen.chord_input_open());
    type_text(&mut screen, "key:G Isus4-I", now, &ctx);
    screen.handle_key(key(KeyCode::Enter), now, &ctx);

    assert!(!screen.chord_input_open());
    assert_eq!(screen.fixed_chord().unwrap().input(), "key:G Isus4-I");
    assert!(!screen.cycle_random().chord);
    assert!(screen.chord_enabled());
    let chord = screen.state.chord().unwrap();
    assert_eq!(chord.key(), "G");
    assert_eq!(chord.degrees(), "Isus4-I");
    assert_eq!(chord.chord_count(), 2);
}

#[test]
fn invalid_input_stays_open_without_changing_playback() {
    let now = Instant::now();
    let (mut screen, patches, catalog) = setup();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), catalog, &PolyPatch);
    screen.start(now, &ctx);
    screen.handle_key(key(KeyCode::Char('i')), now, &ctx);
    type_text(&mut screen, "zzz", now, &ctx);

    screen.handle_key(key(KeyCode::Enter), now, &ctx);

    let overlay = screen.chord_input_overlay().expect("入力を閉じない");
    assert!(overlay.error().is_some());
    assert!(screen.state.chord().is_none());
    assert!(screen.fixed_chord().is_none());
    assert!(screen.cycle_random().chord);
}

#[test]
fn unavailable_poly_patch_rejects_atomically() {
    let now = Instant::now();
    let mut screen = GridSequencerScreen::new(None);
    let ctx = ctx_with(GridPatchLoad::Ready(&[]), empty_catalog(), &PolyPatch);
    screen.start(now, &ctx);
    let before = screen.state.instances().to_vec();
    screen.handle_key(key(KeyCode::Char('i')), now, &ctx);
    type_text(&mut screen, "key:G I-IV", now, &ctx);

    screen.handle_key(key(KeyCode::Enter), now, &ctx);

    assert!(screen.chord_input_overlay().unwrap().error().is_some());
    assert_eq!(screen.state.instances(), before);
    assert!(screen.state.chord().is_none());
    assert!(screen.fixed_chord().is_none());
    assert!(screen.cycle_random().chord);
}

#[test]
fn escape_cancels_without_applying_the_input() {
    let now = Instant::now();
    let (mut screen, patches, catalog) = setup();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), catalog, &PolyPatch);
    screen.start(now, &ctx);
    screen.handle_key(key(KeyCode::Char('i')), now, &ctx);
    type_text(&mut screen, "key:G I", now, &ctx);

    screen.handle_key(key(KeyCode::Esc), now, &ctx);

    assert!(!screen.chord_input_open());
    assert!(screen.state.chord().is_none());
    assert!(screen.fixed_chord().is_none());
}

#[test]
fn fixed_progression_survives_r_upper_r_and_chord_mode_toggle() {
    let now = Instant::now();
    let (mut screen, patches, catalog) = setup();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), catalog, &PolyPatch);
    screen.start(now, &ctx);
    screen.handle_key(key(KeyCode::Char('i')), now, &ctx);
    type_text(&mut screen, "key:G Isus4-I", now, &ctx);
    screen.handle_key(key(KeyCode::Enter), now, &ctx);

    for code in [KeyCode::Char('r'), KeyCode::Char('R')] {
        screen.handle_key(key(code), now, &ctx);
        let chord = screen.state.chord().unwrap();
        assert_eq!((chord.key(), chord.degrees()), ("G", "Isus4-I"));
    }

    screen.handle_key(key(KeyCode::Char('c')), now, &ctx);
    assert!(screen.state.chord().is_none());
    assert!(screen.fixed_chord().is_some());
    screen.handle_key(key(KeyCode::Char('c')), now, &ctx);
    let restored = screen.state.chord().unwrap();
    assert_eq!((restored.key(), restored.degrees()), ("G", "Isus4-I"));
}

#[test]
fn enabling_cycle_random_chord_releases_the_fixed_input() {
    let now = Instant::now();
    let (mut screen, patches, catalog) = setup();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), catalog, &PolyPatch);
    screen.start(now, &ctx);
    screen.handle_key(key(KeyCode::Char('i')), now, &ctx);
    type_text(&mut screen, "key:G Isus4-I", now, &ctx);
    screen.handle_key(key(KeyCode::Enter), now, &ctx);
    let before = screen.state.chord().unwrap().clone();

    screen.handle_key(key(KeyCode::Char('a')), now, &ctx);
    screen.handle_key(key(KeyCode::Char('5')), now, &ctx);

    assert!(screen.cycle_random().chord);
    assert!(screen.fixed_chord().is_none());
    assert_eq!(screen.state.chord(), Some(&before));
}

#[test]
fn reopening_uses_the_original_fixed_text() {
    let now = Instant::now();
    let (mut screen, patches, catalog) = setup();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), catalog, &PolyPatch);
    screen.start(now, &ctx);
    screen.handle_key(key(KeyCode::Char('i')), now, &ctx);
    type_text(&mut screen, "KEY:G♭ I-IV", now, &ctx);
    screen.handle_key(key(KeyCode::Enter), now, &ctx);

    screen.handle_key(key(KeyCode::Char('i')), now, &ctx);

    assert_eq!(screen.chord_input_overlay().unwrap().value(), "KEY:G♭ I-IV");
}
