use std::time::Instant;

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use cmrt_rhythm::{DrumPattern, DrumRole, HatPattern, KickPattern};

use super::*;
use crate::{
    ChordPlayback, LaneAddress, NoteStep, PatternEvolution, FIRST_DRUM_ROW, FULL_DRUM_TRACK_COUNT,
    GRID_STEPS,
};

const AREA: Rect = Rect::new(0, 0, 90, 30);
/// いちばん下（＝ index が最大）の drum 行が kick。
const KICK_ROW: usize = FULL_DRUM_TRACK_COUNT - 1;

fn context() -> crate::GridSequencerContext<'static> {
    crate::tests::ctx_with(
        crate::GridPatchLoad::Ready(&[]),
        crate::tests::empty_catalog(),
        &crate::NoVoicingLookup,
    )
}

fn screen() -> GridSequencerScreen {
    GridSequencerScreen::with_track_count(None, FULL_DRUM_TRACK_COUNT)
}

fn wheel(screen: &GridSequencerScreen, kind: MouseEventKind, instance: usize) -> MouseEvent {
    let layout = crate::ui::layout_for(screen, AREA);
    MouseEvent {
        kind,
        column: layout.step_column(0),
        row: layout.lane_line(
            &screen.state.visible_note_rows(),
            LaneAddress::new(instance, 0),
        ),
        modifiers: KeyModifiers::NONE,
    }
}

fn pattern_text(screen: &GridSequencerScreen, instance: usize) -> String {
    screen
        .state
        .lane(LaneAddress::new(instance, 0))
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

/// 初回の down は役割の list の先頭。kick なら4つ打ち。
#[test]
fn wheel_down_on_a_drum_row_writes_the_first_rhythm_of_its_role() {
    let mut screen = screen();

    screen.handle_mouse(
        wheel(&screen, MouseEventKind::ScrollDown, KICK_ROW),
        AREA,
        &context(),
    );

    assert_eq!(
        screen.last_drum(),
        Some(DrumPattern::Kick(KickPattern::default()))
    );
    // 次の音まで伸ばしっぱなし。
    assert_eq!(pattern_text(&screen, KICK_ROW), "#---#---#---#---");
}

/// 初回の up は list の末尾。
#[test]
fn wheel_up_on_a_drum_row_starts_from_the_end_of_the_list() {
    let mut screen = screen();

    screen.handle_mouse(
        wheel(&screen, MouseEventKind::ScrollUp, KICK_ROW),
        AREA,
        &context(),
    );

    let expected = *DrumPattern::all_for(DrumRole::Kick)
        .last()
        .expect("kick の list は空でない");
    assert_eq!(screen.last_drum(), Some(expected));
}

/// list は役割ごとに分かれている。kick 行を回しても hi-hat の型は出てこない。
#[test]
fn the_list_never_leaves_the_role_of_the_row() {
    let mut screen = screen();

    for _ in 0..8 {
        screen.handle_mouse(
            wheel(&screen, MouseEventKind::ScrollDown, KICK_ROW),
            AREA,
            &context(),
        );
        assert_eq!(
            screen.last_drum().map(DrumPattern::role),
            Some(DrumRole::Kick)
        );
    }
}

/// 行が違えばカーソルも別。hi-hat 行は hi-hat の list を歩く。
#[test]
fn each_row_keeps_its_own_cursor() {
    let mut screen = screen();
    let hat_row = FULL_DRUM_TRACK_COUNT - 3;
    assert_eq!(screen.state.drum_role(hat_row), Some(DrumRole::HiHat));

    screen.handle_mouse(
        wheel(&screen, MouseEventKind::ScrollDown, KICK_ROW),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        wheel(&screen, MouseEventKind::ScrollDown, hat_row),
        AREA,
        &context(),
    );

    assert_eq!(
        screen.last_drum(),
        Some(DrumPattern::Hat(HatPattern::default()))
    );
    assert_eq!(pattern_text(&screen, hat_row), "#-#-#-#-#-#-#-#-");
    // kick 行の譜面はそのまま。
    assert_eq!(pattern_text(&screen, KICK_ROW), "#---#---#---#---");
}

/// chord mode を使わなくても回せる。drum の音高はコードから導出しないため。
#[test]
fn the_wheel_works_without_the_chord_mode() {
    let mut screen = screen();
    assert!(screen.state.chord().is_none());

    screen.handle_mouse(
        wheel(&screen, MouseEventKind::ScrollDown, KICK_ROW),
        AREA,
        &context(),
    );

    assert!(screen.last_drum().is_some());
}

/// chord mode 中でも drum 行はアルペジオではなくリズムを引く。
#[test]
fn the_chord_mode_does_not_route_a_drum_row_to_the_arpeggiator() {
    let mut screen = screen();
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );

    screen.handle_mouse(
        wheel(&screen, MouseEventKind::ScrollDown, KICK_ROW),
        AREA,
        &context(),
    );

    assert_eq!(screen.last_arp(), None);
    assert!(screen.last_drum().is_some());
}

/// 引き直しは手編集なので HOLD へ落ちる。AUTO のままだと1周で消える。
#[test]
fn one_wheel_click_holds_the_pattern_and_is_one_undo_step() {
    let mut screen = screen();
    let before = pattern_text(&screen, KICK_ROW);
    screen.handle_mouse(
        wheel(&screen, MouseEventKind::ScrollDown, KICK_ROW),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        wheel(&screen, MouseEventKind::ScrollDown, KICK_ROW),
        AREA,
        &context(),
    );
    let after_two = pattern_text(&screen, KICK_ROW);
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Hold);

    screen.undo(&context());

    assert_ne!(after_two, before);
    assert_eq!(pattern_text(&screen, KICK_ROW), "#---#---#---#---");
}

/// カーソルは track 数の切替で捨てる。instance 番号の指す役割が変わるため。
#[test]
fn changing_the_track_count_drops_the_pattern_cursor() {
    let mut screen = screen();
    screen.handle_mouse(
        wheel(&screen, MouseEventKind::ScrollDown, KICK_ROW),
        AREA,
        &context(),
    );
    assert!(screen.last_drum().is_some());

    screen.resize_for_restart(4, &[]);

    assert_eq!(screen.last_drum(), None);
}

/// drum 行でない行の wheel は drum の list を動かさない。
#[test]
fn a_row_without_a_drum_role_does_not_touch_the_drum_cursor() {
    let mut screen = screen();

    screen.handle_mouse(
        wheel(&screen, MouseEventKind::ScrollDown, FIRST_DRUM_ROW - 1),
        AREA,
        &context(),
    );

    assert_eq!(screen.last_drum(), None);
}

/// 譜面は 16 step ぶんちょうど埋まる。
#[test]
fn every_written_rhythm_covers_the_whole_measure() {
    let mut screen = screen();

    for _ in 0..DrumPattern::all_for(DrumRole::Kick).len() {
        screen.handle_mouse(
            wheel(&screen, MouseEventKind::ScrollDown, KICK_ROW),
            AREA,
            &context(),
        );
        assert_eq!(pattern_text(&screen, KICK_ROW).chars().count(), GRID_STEPS);
    }
}

/// 右 drag（note 消去）と取り違えないための最低限の確認。
#[test]
fn a_click_is_not_a_wheel_turn() {
    let mut screen = screen();
    let event = wheel(&screen, MouseEventKind::Down(MouseButton::Left), KICK_ROW);

    screen.handle_mouse(event, AREA, &context());

    assert_eq!(screen.last_drum(), None);
}
