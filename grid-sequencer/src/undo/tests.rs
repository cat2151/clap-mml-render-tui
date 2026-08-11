use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::*;
use crate::{tests::ctx_with, ChordPlayback, GridPatchLoad, NoVoicingLookup, NotePattern};

const AREA: Rect = Rect::new(0, 0, 90, 24);

fn mouse(kind: MouseEventKind, column: u16) -> MouseEvent {
    mouse_at(kind, column, 2)
}

/// step セルの列。grid は中央寄せなので、chord 行の有無で左端が動く。
fn cell(screen: &GridSequencerScreen, step: usize) -> u16 {
    crate::ui::layout_for(screen, AREA).step_column(step)
}

/// NOTE 欄（音高）の列。
fn note_column(screen: &GridSequencerScreen) -> u16 {
    crate::ui::layout_for(screen, AREA).note_column()
}

fn mouse_at(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

#[test]
fn one_cross_row_drag_undoes_every_touched_row_together() {
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.handle_mouse(
        mouse_at(MouseEventKind::Down(MouseButton::Left), cell(&screen, 0), 2),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse_at(MouseEventKind::Drag(MouseButton::Left), cell(&screen, 2), 3),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse_at(MouseEventKind::Up(MouseButton::Left), cell(&screen, 2), 3),
        AREA,
        &context(),
    );
    assert!(screen.state.rows()[0].pattern.is_attack(0));
    assert!(screen.state.rows()[1].pattern.is_attack(2));

    screen.handle_key(press('u'), Instant::now(), &context());

    assert_eq!(screen.state.rows()[0].pattern, NotePattern::default());
    assert_eq!(screen.state.rows()[1].pattern, NotePattern::default());
}

#[test]
fn chord_voicing_wheel_is_undoable() {
    // chord ON の行は 3=和音、4=bass、5〜8が 4 voice。
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );
    assert_eq!(
        crate::chord_gains_db(4, true, screen.pattern_evolution())[0],
        crate::CHORD_GAIN_DB
    );

    screen.handle_mouse(
        mouse_at(MouseEventKind::ScrollDown, note_column(&screen), 6),
        AREA,
        &context(),
    );
    assert_eq!(screen.state.instances()[2].voicing_rotation, -1);
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Hold);
    assert_eq!(
        crate::chord_gains_db(4, true, screen.pattern_evolution()),
        vec![0.0; 8]
    );

    screen.handle_key(press('u'), Instant::now(), &context());

    assert_eq!(screen.state.instances()[2].voicing_rotation, 0);
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Auto);
    assert_eq!(
        crate::chord_gains_db(4, true, screen.pattern_evolution())[0],
        crate::CHORD_GAIN_DB
    );
}

fn press(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE)
}

fn context() -> crate::GridSequencerContext<'static> {
    ctx_with(
        GridPatchLoad::Ready(&[]),
        crate::tests::empty_catalog(),
        &NoVoicingLookup,
    )
}

#[test]
fn one_drag_is_undone_as_one_operation_and_restores_auto() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 0)),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(&screen, 1)),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Up(MouseButton::Left), cell(&screen, 1)),
        AREA,
        &context(),
    );
    assert_eq!(screen.state.rows()[0].pattern.attack_len(0), Some(2));
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Hold);

    screen.handle_key(press('u'), Instant::now(), &context());

    assert_eq!(screen.state.rows()[0].pattern, NotePattern::default());
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Auto);
}

#[test]
fn second_drag_replaces_the_single_undo_slot() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    for step in [0, 1] {
        let column = cell(&screen, step);
        screen.handle_mouse(
            mouse(MouseEventKind::Down(MouseButton::Left), column),
            AREA,
            &context(),
        );
        screen.handle_mouse(
            mouse(MouseEventKind::Up(MouseButton::Left), column),
            AREA,
            &context(),
        );
    }

    screen.handle_key(press('u'), Instant::now(), &context());

    assert!(screen.state.rows()[0].pattern.is_attack(0));
    assert!(!screen.state.rows()[0].pattern.is_attack(1));
}

#[test]
fn clear_is_undoable() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].pattern.draw_span(0, 3);
    screen.handle_key(press('x'), Instant::now(), &context());
    screen.handle_key(press('u'), Instant::now(), &context());
    assert_eq!(screen.state.rows()[0].pattern.attack_len(0), Some(4));
}
