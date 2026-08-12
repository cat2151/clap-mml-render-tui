use std::time::Instant;

use crossterm::event::MouseEvent;

use super::*;
use crate::{ChordPlayback, GridRow, LaneAddress, NoteStep};

mod chord;

const AREA: Rect = Rect::new(0, 0, 90, 24);

fn context() -> crate::GridSequencerContext<'static> {
    crate::tests::ctx_with(
        crate::GridPatchLoad::Ready(&[]),
        crate::tests::empty_catalog(),
        &crate::NoVoicingLookup,
    )
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn shifted_mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        modifiers: KeyModifiers::SHIFT,
        ..mouse(kind, column, row)
    }
}

/// step セルの列。grid は中央寄せなので、chord 行の有無で左端が動く。
fn cell(screen: &GridSequencerScreen, step: usize) -> u16 {
    crate::ui::layout_for(screen, AREA).step_column(step)
}

/// NOTE 欄（音高）の列。
fn note_column(screen: &GridSequencerScreen) -> u16 {
    crate::ui::layout_for(screen, AREA).note_column()
}

fn pattern(screen: &GridSequencerScreen, row: usize) -> String {
    screen.state.rows()[row]
        .pattern
        .steps()
        .iter()
        .map(|step| match step {
            NoteStep::Rest => '.',
            NoteStep::Attack => '#',
            NoteStep::Tie => '-',
        })
        .collect()
}

fn lane_pattern(screen: &GridSequencerScreen, instance: usize, lane: usize) -> String {
    screen
        .state
        .lane(LaneAddress::new(instance, lane))
        .unwrap()
        .pattern
        .steps()
        .iter()
        .map(|step| match step {
            NoteStep::Rest => '.',
            NoteStep::Attack => '#',
            NoteStep::Tie => '-',
        })
        .collect()
}

#[test]
fn left_drag_draws_one_long_note_and_shrinks_from_the_down_snapshot() {
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 4), 2),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(&screen, 7), 2),
        AREA,
        &context(),
    );
    assert_eq!(&pattern(&screen, 0)[4..8], "#---");

    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(&screen, 5), 2),
        AREA,
        &context(),
    );
    assert_eq!(&pattern(&screen, 0)[4..8], "#-..");
    assert!(!screen.cycle_random().get(crate::CycleRandomItem::Note));

    screen.handle_mouse(
        mouse(MouseEventKind::Up(MouseButton::Left), cell(&screen, 5), 2),
        AREA,
        &context(),
    );
    assert!(screen.note_gesture.is_none());
}

#[test]
fn shrinking_restores_existing_notes_that_leave_the_drawn_span() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].pattern.draw_span(6, 7);
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 4), 2),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(&screen, 7), 2),
        AREA,
        &context(),
    );
    assert_eq!(&pattern(&screen, 0)[4..8], "#---");

    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(&screen, 5), 2),
        AREA,
        &context(),
    );
    assert_eq!(&pattern(&screen, 0)[4..8], "#-#-");
}

#[test]
fn dragging_left_of_the_anchor_clamps_to_a_one_step_note() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 4), 2),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(&screen, 1), 2),
        AREA,
        &context(),
    );
    assert_eq!(&pattern(&screen, 0)[1..6], "...#.");
}

#[test]
fn draw_gesture_can_cross_rows_and_keeps_each_rows_own_note() {
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 1), 2),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(&screen, 4), 3),
        AREA,
        &context(),
    );
    assert_eq!(pattern(&screen, 0), ".#..............");
    assert_eq!(pattern(&screen, 1), "....#...........");
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(&screen, 3), 2),
        AREA,
        &context(),
    );
    assert_eq!(&pattern(&screen, 0)[1..4], "#--");
    assert_eq!(pattern(&screen, 1), "....#...........");
}

#[test]
fn chord_voice_drag_draws_an_arpeggio_across_skipped_rows() {
    // chord ON の行は 3=和音、4/5=bass(octave 上/root)、6〜9が 4 voice(lane 3〜0)。
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );

    // 下段root(lane0)から上へ飛ばす。中間laneも直線補間したstepへ描く。
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 1), 9),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(&screen, 5), 7),
        AREA,
        &context(),
    );
    assert_eq!(lane_pattern(&screen, 2, 0), ".#..............");
    assert_eq!(lane_pattern(&screen, 2, 1), "...#............");
    assert_eq!(lane_pattern(&screen, 2, 2), ".....#..........");
    assert_eq!(lane_pattern(&screen, 2, 3), "................");
    screen.handle_mouse(
        mouse(MouseEventKind::Up(MouseButton::Left), cell(&screen, 5), 7),
        AREA,
        &context(),
    );

    // triadでoctave上を重ねるlane3も独立patternとして編集できる。
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 6), 6),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Up(MouseButton::Left), cell(&screen, 6), 6),
        AREA,
        &context(),
    );
    assert_eq!(lane_pattern(&screen, 2, 3), "......#.........");
}

#[test]
fn toggling_chord_mode_finishes_an_active_lane_gesture_before_remapping_rows() {
    let now = Instant::now();
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        now,
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 2), 9),
        AREA,
        &context(),
    );
    assert!(screen.note_gesture.is_some());

    screen.toggle_chord_mode(now, &context());

    assert!(screen.note_gesture.is_none());
    assert!(screen.state.chord().is_none());
    assert_eq!(lane_pattern(&screen, 2, 0), "..#.............");
}

#[test]
fn left_down_on_attack_or_tie_erases_the_whole_note() {
    for clicked_step in [2, 3] {
        let mut screen = GridSequencerScreen::with_track_count(None, 1);
        screen.state.rows_mut()[0].pattern.draw_span(2, 4);
        screen.state.rows_mut()[0].pattern.draw_span(7, 7);
        screen.handle_mouse(
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                cell(&screen, clicked_step),
                2,
            ),
            AREA,
            &context(),
        );
        assert_eq!(&pattern(&screen, 0)[2..8], ".....#");
    }
}

#[test]
fn right_drag_erases_each_note_event_it_crosses() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].pattern.draw_span(0, 2);
    screen.state.rows_mut()[0].pattern.draw_span(4, 5);
    screen.handle_mouse(
        mouse(
            MouseEventKind::Down(MouseButton::Right),
            cell(&screen, 1),
            2,
        ),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(
            MouseEventKind::Drag(MouseButton::Right),
            cell(&screen, 4),
            2,
        ),
        AREA,
        &context(),
    );
    assert_eq!(pattern(&screen, 0), "................");
}

#[test]
fn a_new_down_finishes_a_gesture_whose_up_was_lost() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 0), 2),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 1), 2),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(&screen, 2), 2),
        AREA,
        &context(),
    );
    assert_eq!(&pattern(&screen, 0)[..3], "##-");
}

#[test]
fn editing_discards_a_cycle_staged_from_the_old_rows() {
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.state.stage_next_cycle(
        vec![GridRow::default(); 2],
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]).unwrap(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 0), 2),
        AREA,
        &context(),
    );
    assert!(!screen.state.has_pending_cycle());
    assert!(!screen.cycle_random().get(crate::CycleRandomItem::Note));
}

#[test]
fn chord_row_and_overlays_block_mouse_edits() {
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 0), 3),
        AREA,
        &context(),
    );
    assert_eq!(pattern(&screen, 0), "................");

    screen.state.set_chord(None, Instant::now());
    screen.help_open = true;
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 0), 2),
        AREA,
        &context(),
    );
    assert_eq!(pattern(&screen, 0), "................");

    screen.help_open = false;
    screen.waiting_for_patches = true;
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 0), 2),
        AREA,
        &context(),
    );
    assert_eq!(pattern(&screen, 0), "................");
}

#[test]
fn chord_error_line_and_input_use_the_same_vertical_layout() {
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.chord_error = Some("test error".to_string());
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 0), 3),
        AREA,
        &context(),
    );
    assert!(screen.state.rows()[0].pattern.is_attack(0));
}

#[test]
fn wheel_edits_note_and_shift_wheel_edits_octave() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].base_note = 60;
    screen.handle_mouse(
        mouse(MouseEventKind::ScrollUp, note_column(&screen), 2),
        AREA,
        &context(),
    );
    assert_eq!(screen.state.rows()[0].base_note, 61);
    screen.handle_mouse(
        shifted_mouse(MouseEventKind::ScrollUp, note_column(&screen), 2),
        AREA,
        &context(),
    );
    assert_eq!(screen.state.rows()[0].base_note, 73);
}

#[test]
fn clipped_cell_does_not_change_state() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    let narrow = Rect::new(0, 0, 39, 24);
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(&screen, 3), 2),
        narrow,
        &context(),
    );
    assert_eq!(screen.state.rows()[0].pattern, NotePattern::default());
}
