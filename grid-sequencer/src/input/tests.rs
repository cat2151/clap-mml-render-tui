use std::time::Instant;

use crossterm::event::MouseEvent;

use super::*;
use crate::{ChordPlayback, GridRow, LaneAddress, NoteStep};

const AREA: Rect = Rect::new(0, 0, 90, 24);
const FIRST_CELL: (u16, u16) = (37, 2);

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

fn cell(step: usize) -> u16 {
    FIRST_CELL.0 + step as u16 * 2
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
        mouse(MouseEventKind::Down(MouseButton::Left), cell(4), 2),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(7), 2),
        AREA,
        &context(),
    );
    assert_eq!(&pattern(&screen, 0)[4..8], "#---");

    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(5), 2),
        AREA,
        &context(),
    );
    assert_eq!(&pattern(&screen, 0)[4..8], "#-..");
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Hold);

    screen.handle_mouse(
        mouse(MouseEventKind::Up(MouseButton::Left), cell(5), 2),
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
        mouse(MouseEventKind::Down(MouseButton::Left), cell(4), 2),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(7), 2),
        AREA,
        &context(),
    );
    assert_eq!(&pattern(&screen, 0)[4..8], "#---");

    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(5), 2),
        AREA,
        &context(),
    );
    assert_eq!(&pattern(&screen, 0)[4..8], "#-#-");
}

#[test]
fn dragging_left_of_the_anchor_clamps_to_a_one_step_note() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(4), 2),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(1), 2),
        AREA,
        &context(),
    );
    assert_eq!(&pattern(&screen, 0)[1..6], "...#.");
}

#[test]
fn draw_gesture_can_cross_rows_and_keeps_each_rows_own_note() {
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(1), 2),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(4), 3),
        AREA,
        &context(),
    );
    assert_eq!(pattern(&screen, 0), ".#..............");
    assert_eq!(pattern(&screen, 1), "....#...........");
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(3), 2),
        AREA,
        &context(),
    );
    assert_eq!(&pattern(&screen, 0)[1..4], "#--");
    assert_eq!(pattern(&screen, 1), "....#...........");
}

#[test]
fn chord_voice_drag_draws_an_arpeggio_across_skipped_rows() {
    // chord ON の行は 3=和音、4=bass、5〜8が 4 voice(lane 3〜0)。
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );

    // 下段root(lane0)から上へ飛ばす。中間laneも直線補間したstepへ描く。
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(1), 8),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(5), 6),
        AREA,
        &context(),
    );
    assert_eq!(lane_pattern(&screen, 2, 0), ".#..............");
    assert_eq!(lane_pattern(&screen, 2, 1), "...#............");
    assert_eq!(lane_pattern(&screen, 2, 2), ".....#..........");
    assert_eq!(lane_pattern(&screen, 2, 3), "................");
    screen.handle_mouse(
        mouse(MouseEventKind::Up(MouseButton::Left), cell(5), 6),
        AREA,
        &context(),
    );

    // triadでoctave上を重ねるlane3も独立patternとして編集できる。
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(6), 5),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Up(MouseButton::Left), cell(6), 5),
        AREA,
        &context(),
    );
    assert_eq!(lane_pattern(&screen, 2, 3), "......#.........");
}

#[test]
fn repeated_chord_voice_wheel_down_accumulates_and_survives_chord_changes() {
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    let before = screen.state.instances()[2]
        .lanes
        .iter()
        .map(|lane| lane.base_note)
        .collect::<Vec<_>>();
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );
    screen.handle_mouse(mouse(MouseEventKind::ScrollDown, 32, 6), AREA, &context());
    assert_eq!(screen.state.instances()[2].voicing_rotation, -1);
    assert_eq!(
        (0..4)
            .map(|lane| screen.state.resolved_note(LaneAddress::new(2, lane)))
            .collect::<Vec<_>>(),
        vec![Some(55), Some(60), Some(64), Some(67)]
    );
    assert_eq!(
        screen.state.instances()[2]
            .lanes
            .iter()
            .map(|lane| lane.base_note)
            .collect::<Vec<_>>(),
        before
    );

    // 高速wheel相当の連続downでもroot positionへwrapせず、そのまま下がり続ける。
    screen.handle_mouse(mouse(MouseEventKind::ScrollDown, 32, 6), AREA, &context());
    screen.handle_mouse(mouse(MouseEventKind::ScrollDown, 32, 6), AREA, &context());
    assert_eq!(screen.state.instances()[2].voicing_rotation, -3);
    assert_eq!(
        (0..4)
            .map(|lane| screen.state.resolved_note(LaneAddress::new(2, lane)))
            .collect::<Vec<_>>(),
        vec![Some(48), Some(52), Some(55), Some(60)]
    );

    screen.state.set_chord(
        ChordPlayback::new("D", "I".to_string(), vec![vec![62, 66, 69]]),
        Instant::now(),
    );
    assert_eq!(screen.state.instances()[2].voicing_rotation, -3);
    assert_eq!(
        (0..4)
            .map(|lane| screen.state.resolved_note(LaneAddress::new(2, lane)))
            .collect::<Vec<_>>(),
        vec![Some(50), Some(54), Some(57), Some(62)]
    );

    screen.state.set_chord(None, Instant::now());
    // chord OFF では chord 行が消えて行が1つ繰り上がる。行4が 4 voice の instance。
    screen.handle_mouse(mouse(MouseEventKind::ScrollUp, 32, 4), AREA, &context());
    assert_eq!(
        screen.state.instances()[2].lanes[0].base_note,
        before[0] + 1
    );
    assert_eq!(
        screen.state.instances()[2].lanes[1..]
            .iter()
            .map(|lane| lane.base_note)
            .collect::<Vec<_>>(),
        before[1..]
    );
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
        mouse(MouseEventKind::Down(MouseButton::Left), cell(2), 8),
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
                cell(clicked_step),
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
        mouse(MouseEventKind::Down(MouseButton::Right), cell(1), 2),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Right), cell(4), 2),
        AREA,
        &context(),
    );
    assert_eq!(pattern(&screen, 0), "................");
}

#[test]
fn a_new_down_finishes_a_gesture_whose_up_was_lost() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(0), 2),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(1), 2),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), cell(2), 2),
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
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            FIRST_CELL.0,
            FIRST_CELL.1,
        ),
        AREA,
        &context(),
    );
    assert!(!screen.state.has_pending_cycle());
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Hold);
}

#[test]
fn chord_row_and_overlays_block_mouse_edits() {
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(0), 3),
        AREA,
        &context(),
    );
    assert_eq!(pattern(&screen, 0), "................");

    screen.state.set_chord(None, Instant::now());
    screen.help_open = true;
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(0), 2),
        AREA,
        &context(),
    );
    assert_eq!(pattern(&screen, 0), "................");

    screen.help_open = false;
    screen.waiting_for_patches = true;
    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), cell(0), 2),
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
        mouse(MouseEventKind::Down(MouseButton::Left), cell(0), 3),
        AREA,
        &context(),
    );
    assert!(screen.state.rows()[0].pattern.is_attack(0));
}

#[test]
fn wheel_edits_note_and_shift_wheel_edits_octave() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].base_note = 60;
    screen.handle_mouse(mouse(MouseEventKind::ScrollUp, 32, 2), AREA, &context());
    assert_eq!(screen.state.rows()[0].base_note, 61);
    screen.handle_mouse(
        shifted_mouse(MouseEventKind::ScrollUp, 32, 2),
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
        mouse(MouseEventKind::Down(MouseButton::Left), cell(3), 2),
        narrow,
        &context(),
    );
    assert_eq!(screen.state.rows()[0].pattern, NotePattern::default());
}
