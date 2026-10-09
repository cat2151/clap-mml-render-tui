use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::{DrumHit, DEFAULT_VELOCITY};

fn hit(note: u8, step: usize, steps: u8) -> DrumHit {
    DrumHit {
        note,
        step,
        steps,
        velocity: DEFAULT_VELOCITY,
    }
}

fn press(screen: &mut DrumSequencerScreen, code: KeyCode) {
    screen.handle_key_event(KeyEvent::new(code, KeyModifiers::NONE));
}

#[test]
fn kit_changes_preserve_note_inputs_and_cursor_step() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit(
        "first".into(),
        Some(vec![60, 36, 60, 42]),
        Vec::new(),
        Vec::new(),
    );
    assert_eq!(screen.notes(), [36, 42, 60]);
    // 最初は最も低い note。上へ 1 行で 42。
    press(&mut screen, KeyCode::Char('k'));
    press(&mut screen, KeyCode::Char('l'));
    press(&mut screen, KeyCode::Char(' '));
    assert_eq!(screen.cursor_note(), Some(42));
    assert!(screen.cell_on(42, 1));

    // 42 の行番号が変わっても、同じ note のカーソルと入力を維持する。
    screen.set_kit("second".into(), Some(vec![70, 42]), Vec::new(), Vec::new());
    assert_eq!(screen.cursor_note(), Some(42));
    assert_eq!(screen.cursor_step(), 1);
    assert!(screen.cell_on(42, 1));
    assert!(!screen.cell_on(70, 1));

    // 非表示 note は消さず、現在の note が消えたら最も低い note を選ぶ。
    screen.set_kit("third".into(), Some(vec![80, 70]), Vec::new(), Vec::new());
    assert_eq!(screen.cursor_note(), Some(70));
    assert_eq!(screen.cursor_step(), 1);
    assert!(screen.cell_on(42, 1));
    press(&mut screen, KeyCode::Char(' '));
    assert!(screen.cell_on(70, 1));

    screen.set_kit(
        "first".into(),
        Some(vec![60, 42, 36]),
        Vec::new(),
        Vec::new(),
    );
    assert_eq!(screen.cursor_note(), Some(36));
    assert_eq!(screen.cursor_step(), 1);
    assert!(screen.cell_on(42, 1));
    assert!(!screen.cell_on(36, 1));
    // 表示対象は現在の kit の一覧だけで、隠れた 70 の入力も残っている。
    assert!(!screen.notes().contains(&70));
    assert!(screen.cell_on(70, 1));
}

#[test]
fn unselected_unknown_and_empty_kits_do_not_edit_or_move() {
    let mut screen = DrumSequencerScreen::default();
    assert_eq!(screen.kit_name(), None);
    assert!(!screen.notes_known());
    for stage in 0..3 {
        if stage > 0 {
            screen.set_kit(
                "empty".into(),
                if stage == 1 { None } else { Some(vec![]) },
                Vec::new(),
                Vec::new(),
            );
        }
        for code in [
            KeyCode::Char('h'),
            KeyCode::Char('j'),
            KeyCode::Char('k'),
            KeyCode::Char('l'),
            KeyCode::Left,
            KeyCode::Down,
            KeyCode::Up,
            KeyCode::Right,
            KeyCode::Char(' '),
            KeyCode::Enter,
        ] {
            press(&mut screen, code);
        }
        assert_eq!(screen.cursor_note(), None);
        assert_eq!(screen.cursor_step(), 0);
        assert!(screen.current_pattern().is_empty());
        assert_eq!(screen.take_edited_pattern(), None);
    }
    assert!(screen.notes_known());
    assert!(screen.notes().is_empty());
}

#[test]
fn note_names_follow_their_note_and_are_replaced_with_the_kit() {
    let mut screen = DrumSequencerScreen::default();
    assert_eq!(screen.note_name(36), None);
    screen.set_kit(
        "named".into(),
        Some(vec![42, 36, 38]),
        vec![(42, "Hat".into()), (36, "Kick".into())],
        Vec::new(),
    );
    assert_eq!(screen.note_name(36), Some("Kick"));
    assert_eq!(screen.note_name(42), Some("Hat"));
    assert_eq!(screen.note_name(38), None);

    // 名前の無い kit へ替えたら、前の kit の名前を流用しない。
    screen.set_kit("unnamed".into(), Some(vec![36]), Vec::new(), Vec::new());
    assert_eq!(screen.note_name(36), None);
}

#[test]
fn restored_inputs_wait_for_the_kit_and_keep_the_cursor_note_until_rows_exist() {
    let mut screen = DrumSequencerScreen::default();
    let saved = DrumPattern::from_hits([hit(36, 0, 1), hit(42, 15, 16), hit(99, 3, 2)]);
    screen.restore(
        Some("saved".into()),
        [(3, saved.clone()), (PATTERN_COUNT, saved.clone())],
        PATTERN_COUNT + 4,
        Some(42),
        40,
    );
    assert_eq!(screen.kit_name(), Some("saved"));
    assert_eq!(screen.kit_resolution(), Some(KitResolution::Waiting));
    assert!(screen.notes().is_empty());
    // 範囲外の pattern は捨て、pattern 番号と step は範囲へ収める。
    assert_eq!(screen.pattern_at(3), Some(&saved));
    assert_eq!(screen.pattern_index(), PATTERN_COUNT - 1);
    assert!(screen.current_pattern().is_empty());
    assert_eq!(screen.take_edited_pattern(), None);
    assert_eq!(screen.cursor_step(), DRUM_STEPS - 1);
    assert_eq!(screen.saved_cursor_note(), Some(42));

    // 行の無い kit を挟んでも、保存するカーソル note は失わない。
    screen.mark_kit_missing();
    assert_eq!(screen.kit_resolution(), Some(KitResolution::Missing));
    screen.set_kit("old catalog".into(), None, Vec::new(), Vec::new());
    assert_eq!(screen.kit_resolution(), None);
    assert_eq!(screen.saved_cursor_note(), Some(42));

    screen.set_kit("kit".into(), Some(vec![36, 42, 46]), Vec::new(), Vec::new());
    assert_eq!(screen.cursor_note(), Some(42));
    assert_eq!(screen.cursor_step(), DRUM_STEPS - 1);
    press(&mut screen, KeyCode::Char(']'));
    press(&mut screen, KeyCode::Char(']'));
    press(&mut screen, KeyCode::Char('['));
    // 端から回って 0、次に 1 へ進んで 0 へ戻る。
    assert_eq!(screen.pattern_index(), 0);
    screen.shift_pattern(3);
    assert_eq!(screen.cell_length(99, 3), Some(2));

    // 戻したカーソル note が kit に無ければ、最も低い note。
    let mut screen = DrumSequencerScreen::default();
    screen.restore(Some("saved".into()), [], 0, Some(50), 2);
    screen.set_kit("kit".into(), Some(vec![38, 36]), Vec::new(), Vec::new());
    assert_eq!(screen.cursor_note(), Some(36));
    assert_eq!(screen.cursor_step(), 2);
    // 一度行に置いたら、戻した note は次の kit で使わない。
    screen.set_kit("other".into(), Some(vec![36, 50]), Vec::new(), Vec::new());
    assert_eq!(screen.cursor_note(), Some(36));
}

#[test]
fn one_shot_notes_start_as_a_sixteenth_and_lengths_change_only_on_cells() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".into(), Some(vec![36, 49]), Vec::new(), vec![36, 36]);
    assert!(screen.is_one_shot(36) && !screen.is_one_shot(49));
    press(&mut screen, KeyCode::Char('l'));
    press(&mut screen, KeyCode::Char(' '));
    assert_eq!(screen.cell_length(36, 1), Some(1));
    assert_eq!(screen.take_edited_pattern(), Some(0));
    assert_eq!(screen.take_edited_pattern(), None);
    press(&mut screen, KeyCode::Char('-'));
    // 1 step より短くならず、変わらなければ保存も求めない。
    assert_eq!(screen.cell_length(36, 1), Some(1));
    assert_eq!(screen.take_edited_pattern(), None);
    for _ in 0..3 {
        press(&mut screen, KeyCode::Char('+'));
    }
    press(&mut screen, KeyCode::Char('='));
    assert_eq!(screen.cell_length(36, 1), Some(5));

    // one-shot でない note は 4 分音符で始まり、1 小節より長くならない。
    press(&mut screen, KeyCode::Char('k'));
    press(&mut screen, KeyCode::Char(']'));
    press(&mut screen, KeyCode::Enter);
    assert_eq!(screen.cell_length(49, 1), Some(4));
    assert_eq!(screen.take_edited_pattern(), Some(1));
    for _ in 0..DRUM_STEPS {
        press(&mut screen, KeyCode::Char('+'));
    }
    assert_eq!(screen.cell_length(49, 1), Some(DRUM_STEPS as u8));
    press(&mut screen, KeyCode::Char('-'));
    assert_eq!(screen.cell_length(49, 1), Some(DRUM_STEPS as u8 - 1));

    // pattern ごとに別の入力。
    assert_eq!(screen.pattern_at(0).unwrap().length(49, 1), None);
    assert_eq!(screen.pattern_at(0).unwrap().length(36, 1), Some(5));
}

#[test]
fn only_turning_a_cell_on_is_reported_once() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".into(), Some(vec![36, 42]), Vec::new(), vec![42]);
    press(&mut screen, KeyCode::Char(' '));
    assert_eq!(screen.take_turned_on(), Some(hit(36, 0, 4)));
    assert_eq!(screen.take_turned_on(), None);
    // 長さの変更・OFF・pattern 切替では出ない。
    press(&mut screen, KeyCode::Char('-'));
    press(&mut screen, KeyCode::Char(' '));
    press(&mut screen, KeyCode::Char(']'));
    assert_eq!(screen.take_turned_on(), None);
    // one-shot の note は 1 step。
    press(&mut screen, KeyCode::Char('k'));
    press(&mut screen, KeyCode::Enter);
    assert_eq!(screen.take_turned_on(), Some(hit(42, 0, 1)));
}

#[test]
fn length_and_velocity_keys_on_an_off_cell_snap_to_the_nearest_hit_in_the_row() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".into(), Some(vec![36, 38]), Vec::new(), Vec::new());
    screen.set_patterns([(0, DrumPattern::from_hits([hit(36, 2, 4), hit(36, 8, 4)]))]);
    // 行に x が無ければ、カーソルも入力も変えない。
    press(&mut screen, KeyCode::Char('k'));
    press(&mut screen, KeyCode::Char('+'));
    assert_eq!((screen.cursor_note(), screen.cursor_step()), (Some(38), 0));
    assert_eq!(screen.take_edited_pattern(), None);

    // 右に 1 step ずれた位置から、左の x へ移ってすぐ伸ばす。
    press(&mut screen, KeyCode::Char('j'));
    for _ in 0..3 {
        press(&mut screen, KeyCode::Char('l'));
    }
    press(&mut screen, KeyCode::Char('+'));
    assert_eq!(screen.cursor_step(), 2);
    assert_eq!(screen.cell_length(36, 2), Some(5));
    assert_eq!(screen.take_edited_pattern(), Some(0));
    // 等距離（2 と 8 の間の 5）は左。
    for _ in 0..3 {
        press(&mut screen, KeyCode::Char('l'));
    }
    press(&mut screen, KeyCode::Char('-'));
    assert_eq!(screen.cursor_step(), 2);
    assert_eq!(screen.cell_length(36, 2), Some(4));
    // 右の方が近ければ右。velocity のキーも同じく移る。
    for _ in 0..4 {
        press(&mut screen, KeyCode::Char('l'));
    }
    press(&mut screen, KeyCode::Char(','));
    assert_eq!(screen.cursor_step(), 8);
    assert_eq!(
        screen.cell_hit(36, 8).unwrap().velocity,
        DEFAULT_VELOCITY - 8
    );
    assert_eq!(screen.cell_hit(36, 2).unwrap().velocity, DEFAULT_VELOCITY);
}

#[test]
fn velocity_keys_step_by_eight_within_one_to_127_and_keep_the_length() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".into(), Some(vec![36]), Vec::new(), Vec::new());
    press(&mut screen, KeyCode::Char(' '));
    assert_eq!(screen.take_edited_pattern(), Some(0));
    // 127 から上がらず、変わらなければ保存も求めない。
    press(&mut screen, KeyCode::Char('.'));
    assert_eq!(screen.cell_hit(36, 0), Some(hit(36, 0, 4)));
    assert_eq!(screen.take_edited_pattern(), None);
    for _ in 0..16 {
        screen.handle_key_event(KeyEvent::new_with_kind(
            KeyCode::Char(','),
            KeyModifiers::NONE,
            crossterm::event::KeyEventKind::Repeat,
        ));
    }
    let hit_now = screen.cell_hit(36, 0).unwrap();
    assert_eq!((hit_now.steps, hit_now.velocity), (4, 1));
    assert_eq!(screen.take_edited_pattern(), Some(0));
    press(&mut screen, KeyCode::Char('.'));
    assert_eq!(screen.cell_hit(36, 0).unwrap().velocity, 9);
    // velocity の変更では、再生中に 1 回鳴らす対象にならない。
    screen.take_turned_on();
    press(&mut screen, KeyCode::Char('.'));
    assert_eq!(screen.take_turned_on(), None);
}
