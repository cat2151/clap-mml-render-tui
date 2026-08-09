use std::time::Instant;

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::*;
use crate::{ChordPlayback, LaneAddress, NoteStep, PatternEvolution, GRID_STEPS};

const AREA: Rect = Rect::new(0, 0, 90, 24);
/// NOTE grid の step セル領域の左端。ここより左は PATCH / NOTE 欄。
const FIRST_CELL_COLUMN: u16 = 37;
/// chord mode 中の行番号。行3が和音、行4が bass、行5〜8が 4 voice(lane 3〜0)、
/// 行9が Single lane。
const CHORD_SUMMARY_ROW: u16 = 3;
const TOP_VOICE_ROW: u16 = 5;
const SINGLE_LANE_ROW: u16 = 9;

fn context() -> crate::GridSequencerContext<'static> {
    crate::tests::ctx_with(
        crate::GridPatchLoad::Ready(&[]),
        crate::tests::empty_catalog(),
        &crate::NoVoicingLookup,
    )
}

fn cell(step: usize) -> u16 {
    FIRST_CELL_COLUMN + step as u16 * 2
}

fn wheel(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

/// 和音・bass・4 voice・Single lane の instance を1つずつ持つ画面。
fn chorded_screen() -> GridSequencerScreen {
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );
    screen
}

fn lane_pattern(screen: &GridSequencerScreen, instance: usize, lane: usize) -> String {
    screen
        .state
        .lane(LaneAddress::new(instance, lane))
        .expect("lane exists")
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

fn is_silent(screen: &GridSequencerScreen, instance: usize) -> bool {
    (0..screen.state.instances()[instance].lanes.len()).all(|lane| {
        lane_pattern(screen, instance, lane)
            .chars()
            .all(|c| c == '.')
    })
}

#[test]
fn wheel_up_on_a_step_cell_writes_an_up_arpeggio_across_the_four_voices() {
    let mut screen = chorded_screen();
    screen.handle_mouse(
        wheel(MouseEventKind::ScrollUp, cell(0), TOP_VOICE_ROW),
        AREA,
        &context(),
    );

    assert_eq!(screen.last_arp(), Some(ArpPattern::Up));
    // up arpeggio は lane 0,1,2,3 を順に叩くので、lane L の attack は step ≡ L (mod 4)。
    for lane in 0..4 {
        let pattern = lane_pattern(&screen, 2, lane);
        for step in 0..GRID_STEPS {
            assert_eq!(
                pattern.as_bytes()[step] == b'#',
                step % 4 == lane,
                "lane {lane} step {step}: {pattern}"
            );
        }
    }
}

#[test]
fn generating_an_arpeggio_moves_the_screen_into_hold() {
    let mut screen = chorded_screen();
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Auto);
    screen.handle_mouse(
        wheel(MouseEventKind::ScrollUp, cell(3), TOP_VOICE_ROW),
        AREA,
        &context(),
    );
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Hold);
}

#[test]
fn the_wheel_sends_the_pattern_cursor_in_both_directions() {
    let mut screen = chorded_screen();
    for expected in ArpPattern::ALL {
        screen.handle_mouse(
            wheel(MouseEventKind::ScrollUp, cell(0), TOP_VOICE_ROW),
            AREA,
            &context(),
        );
        assert_eq!(screen.last_arp(), Some(expected));
    }
    // 1周したら先頭へ戻る。
    screen.handle_mouse(
        wheel(MouseEventKind::ScrollUp, cell(0), TOP_VOICE_ROW),
        AREA,
        &context(),
    );
    assert_eq!(screen.last_arp(), Some(ArpPattern::Up));

    for expected in ArpPattern::ALL.iter().rev() {
        screen.handle_mouse(
            wheel(MouseEventKind::ScrollDown, cell(0), TOP_VOICE_ROW),
            AREA,
            &context(),
        );
        assert_eq!(screen.last_arp(), Some(*expected));
    }
}

#[test]
fn the_first_wheel_down_starts_from_the_end_of_the_list() {
    let mut screen = chorded_screen();
    screen.handle_mouse(
        wheel(MouseEventKind::ScrollDown, cell(0), TOP_VOICE_ROW),
        AREA,
        &context(),
    );
    assert_eq!(screen.last_arp(), ArpPattern::ALL.last().copied());
}

#[test]
fn the_chord_summary_row_is_not_arpeggiated() {
    let mut screen = chorded_screen();
    screen.handle_mouse(
        wheel(MouseEventKind::ScrollUp, cell(0), CHORD_SUMMARY_ROW),
        AREA,
        &context(),
    );
    assert_eq!(screen.last_arp(), None);
    assert!(is_silent(&screen, 0));
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Auto);
}

#[test]
fn a_single_lane_row_has_too_few_voices_to_arpeggiate() {
    let mut screen = chorded_screen();
    screen.handle_mouse(
        wheel(MouseEventKind::ScrollUp, cell(0), SINGLE_LANE_ROW),
        AREA,
        &context(),
    );
    assert_eq!(screen.last_arp(), None);
    assert!(is_silent(&screen, 3));
}

#[test]
fn without_a_chord_the_step_cell_wheel_does_nothing() {
    let mut screen = GridSequencerScreen::with_track_count(None, 3);
    // chord OFF では 4 voice instance も1 rowしか出ないので、行3が instance 1。
    screen.handle_mouse(
        wheel(MouseEventKind::ScrollUp, cell(0), 3),
        AREA,
        &context(),
    );
    assert_eq!(screen.last_arp(), None);
    assert!(is_silent(&screen, 1));
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Auto);
}

#[test]
fn one_wheel_click_is_one_undo_step() {
    let mut screen = chorded_screen();
    screen.state.draw_note_span(LaneAddress::new(2, 0), 0, 1);
    let before = (0..4)
        .map(|lane| lane_pattern(&screen, 2, lane))
        .collect::<Vec<_>>();

    screen.handle_mouse(
        wheel(MouseEventKind::ScrollUp, cell(0), TOP_VOICE_ROW),
        AREA,
        &context(),
    );
    assert_ne!(
        (0..4)
            .map(|lane| lane_pattern(&screen, 2, lane))
            .collect::<Vec<_>>(),
        before
    );

    screen.undo(&context());
    assert_eq!(
        (0..4)
            .map(|lane| lane_pattern(&screen, 2, lane))
            .collect::<Vec<_>>(),
        before
    );
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Auto);
}

#[test]
fn the_note_column_wheel_still_rotates_the_voicing() {
    let mut screen = chorded_screen();
    // NOTE 欄（column 32）は従来どおり転回。アルペジオは生成しない。
    screen.handle_mouse(
        wheel(MouseEventKind::ScrollDown, 32, TOP_VOICE_ROW),
        AREA,
        &context(),
    );
    assert_eq!(screen.state.instances()[2].voicing_rotation, -1);
    assert_eq!(screen.last_arp(), None);
}

#[test]
fn changing_the_track_count_drops_the_pattern_cursor() {
    let mut screen = chorded_screen();
    screen.handle_mouse(
        wheel(MouseEventKind::ScrollUp, cell(0), TOP_VOICE_ROW),
        AREA,
        &context(),
    );
    assert_eq!(screen.last_arp(), Some(ArpPattern::Up));

    screen.resize_for_restart(2, &[]);
    assert_eq!(screen.last_arp(), None);
    assert!(screen.arp_patterns.is_empty());
}

#[test]
fn a_left_click_on_a_step_cell_still_draws_a_note() {
    let mut screen = chorded_screen();
    screen.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: cell(2),
            row: TOP_VOICE_ROW,
            modifiers: KeyModifiers::NONE,
        },
        AREA,
        &context(),
    );
    assert_eq!(lane_pattern(&screen, 2, 3), "..#.............");
    assert_eq!(screen.last_arp(), None);
}
