use std::time::Instant;

use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::*;
use crate::{ChordPlayback, CycleRandomItem, LaneAddress, NoteStep, BASS_ROW};

const AREA: Rect = Rect::new(0, 0, 90, 24);
/// chord mode 中の行番号。行3が和音、行4/5が bass(octave 上/root)、
/// 行6〜9が 4 voice、行10が Single lane。
const BASS_OCTAVE_ROW: u16 = 4;
const BASS_ROOT_ROW: u16 = 5;
const TOP_VOICE_ROW: u16 = 6;

fn context() -> crate::GridSequencerContext<'static> {
    crate::tests::ctx_with(
        crate::GridPatchLoad::Ready(&[]),
        crate::tests::empty_catalog(),
        &crate::NoVoicingLookup,
    )
}

/// step セルの列。grid は中央寄せなので、chord 行の有無で左端が動く。
fn cell(screen: &GridSequencerScreen, step: usize) -> u16 {
    crate::ui::layout_for(screen, AREA).step_column(step)
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

fn lane_pattern(screen: &GridSequencerScreen, lane: usize) -> String {
    screen
        .state
        .lane(LaneAddress::new(BASS_ROW, lane))
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

fn scroll(screen: &mut GridSequencerScreen, kind: MouseEventKind, row: u16) {
    let column = cell(screen, 0);
    screen.handle_mouse(wheel(kind, column, row), AREA, &context());
}

#[test]
fn wheel_down_on_the_bass_row_writes_a_whole_note_first() {
    let mut screen = chorded_screen();
    scroll(&mut screen, MouseEventKind::ScrollDown, BASS_ROOT_ROW);

    assert_eq!(screen.last_bass(), Some(BassPattern::Whole));
    assert_eq!(lane_pattern(&screen, 0), "#---------------");
    assert_eq!(lane_pattern(&screen, 1), "................");
}

#[test]
fn the_octave_lane_row_drives_the_same_cursor() {
    let mut screen = chorded_screen();
    // 表示上は octave 上が上段だが、行のどのセルから回しても bass 行1つぶんの操作。
    scroll(&mut screen, MouseEventKind::ScrollDown, BASS_OCTAVE_ROW);
    scroll(&mut screen, MouseEventKind::ScrollDown, BASS_ROOT_ROW);
    scroll(&mut screen, MouseEventKind::ScrollDown, BASS_OCTAVE_ROW);

    assert_eq!(screen.last_bass(), Some(BassPattern::EighthOctave));
    assert_eq!(lane_pattern(&screen, 0), "#-..#-..#-..#-..");
    assert_eq!(lane_pattern(&screen, 1), "..#-..#-..#-..#-");
}

#[test]
fn the_wheel_sends_the_pattern_cursor_in_both_directions() {
    let mut screen = chorded_screen();
    for expected in BassPattern::ALL {
        scroll(&mut screen, MouseEventKind::ScrollDown, BASS_ROOT_ROW);
        assert_eq!(screen.last_bass(), Some(expected));
    }
    // 1周したら先頭へ戻る。
    scroll(&mut screen, MouseEventKind::ScrollDown, BASS_ROOT_ROW);
    assert_eq!(screen.last_bass(), Some(BassPattern::Whole));

    for expected in BassPattern::ALL.iter().rev() {
        scroll(&mut screen, MouseEventKind::ScrollUp, BASS_ROOT_ROW);
        assert_eq!(screen.last_bass(), Some(*expected));
    }
}

#[test]
fn the_first_wheel_up_starts_from_the_end_of_the_list() {
    let mut screen = chorded_screen();
    scroll(&mut screen, MouseEventKind::ScrollUp, BASS_ROOT_ROW);
    assert_eq!(screen.last_bass(), BassPattern::ALL.last().copied());
    // `####` の octave 版。切替は八分ごとなので root・root・octave・octave。
    assert_eq!(lane_pattern(&screen, 0), "##..##..##..##..");
    assert_eq!(lane_pattern(&screen, 1), "..##..##..##..##");
}

#[test]
fn generating_a_bass_line_stops_the_arp_random() {
    let mut screen = chorded_screen();
    assert!(screen.cycle_random().get(CycleRandomItem::Arp));
    scroll(&mut screen, MouseEventKind::ScrollDown, BASS_ROOT_ROW);
    assert!(!screen.cycle_random().get(CycleRandomItem::Arp));
}

#[test]
fn one_wheel_click_is_one_undo_step() {
    let mut screen = chorded_screen();
    screen
        .state
        .draw_note_span(LaneAddress::new(BASS_ROW, 0), 0, 1);
    let before = (0..2)
        .map(|lane| lane_pattern(&screen, lane))
        .collect::<Vec<_>>();

    scroll(&mut screen, MouseEventKind::ScrollDown, BASS_ROOT_ROW);
    assert_ne!(
        (0..2)
            .map(|lane| lane_pattern(&screen, lane))
            .collect::<Vec<_>>(),
        before
    );

    screen.undo(&context());
    assert_eq!(
        (0..2)
            .map(|lane| lane_pattern(&screen, lane))
            .collect::<Vec<_>>(),
        before
    );
    assert!(screen.cycle_random().get(CycleRandomItem::Arp));
}

#[test]
fn without_a_chord_the_bass_row_wheel_does_nothing() {
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    // chord OFF では bass 行も1 rowしか出ないので、行3が bass 行。
    scroll(&mut screen, MouseEventKind::ScrollDown, 3);
    assert_eq!(screen.last_bass(), None);
    assert_eq!(lane_pattern(&screen, 0), "................");
    assert!(screen.cycle_random().get(CycleRandomItem::Arp));
}

#[test]
fn the_four_voice_row_still_gets_an_arpeggio() {
    let mut screen = chorded_screen();
    scroll(&mut screen, MouseEventKind::ScrollDown, TOP_VOICE_ROW);
    assert_eq!(screen.last_bass(), None);
    assert_eq!(screen.last_arp(), Some(crate::ArpPattern::Up));
}

#[test]
fn changing_the_track_count_drops_the_pattern_cursor() {
    let mut screen = chorded_screen();
    scroll(&mut screen, MouseEventKind::ScrollDown, BASS_ROOT_ROW);
    assert_eq!(screen.last_bass(), Some(BassPattern::Whole));

    screen.resize_for_restart(2, &[]);
    assert_eq!(screen.last_bass(), None);
    assert_eq!(screen.bass_pattern, None);
}
