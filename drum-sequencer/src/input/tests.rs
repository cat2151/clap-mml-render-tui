use super::*;
use crate::PATTERN_COUNT;

fn key(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
    KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind)
}

/// (左, 右, 上, 下) の組。hjkl とカーソルキーは同じ移動になる。
const MOVE_KEYS: [[KeyCode; 4]; 2] = [
    [
        KeyCode::Char('h'),
        KeyCode::Char('l'),
        KeyCode::Char('k'),
        KeyCode::Char('j'),
    ],
    [KeyCode::Left, KeyCode::Right, KeyCode::Up, KeyCode::Down],
];

#[test]
fn hjkl_and_arrows_reach_every_note_and_step_and_stop_at_edges() {
    for [left, right, up, down] in MOVE_KEYS {
        let mut screen = DrumSequencerScreen::default();
        screen.set_kit(
            "full range".into(),
            Some((0..128).collect()),
            Vec::new(),
            Vec::new(),
        );
        for _ in 0..3 {
            assert!(screen.handle_key_event(key(left, KeyEventKind::Press)));
            assert!(screen.handle_key_event(key(down, KeyEventKind::Press)));
        }
        assert_eq!(screen.cursor_note(), Some(0));
        assert_eq!(screen.cursor_step(), 0);

        // 上へ動くほど高い note になる（画面は高い note が上）。
        for note in 0..128 {
            assert_eq!(screen.cursor_note(), Some(note));
            for step in 0..DRUM_STEPS {
                assert_eq!(screen.cursor_step(), step);
                screen.handle_key_event(key(right, KeyEventKind::Repeat));
            }
            assert_eq!(screen.cursor_step(), 15);
            for _ in 0..20 {
                screen.handle_key_event(key(left, KeyEventKind::Repeat));
            }
            screen.handle_key_event(key(up, KeyEventKind::Repeat));
        }
        assert_eq!(screen.cursor_note(), Some(127));
        for _ in 0..130 {
            screen.handle_key_event(key(down, KeyEventKind::Repeat));
        }
        assert_eq!(screen.cursor_note(), Some(0));
        for code in [left, right, up, down] {
            screen.handle_key_event(key(code, KeyEventKind::Release));
        }
        assert_eq!((screen.cursor_note(), screen.cursor_step()), (Some(0), 0));
    }
}

#[test]
fn space_toggles_once_per_press_and_ignores_repeat_and_release() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".into(), Some(vec![36]), Vec::new(), Vec::new());
    let code = KeyCode::Char(' ');
    screen.handle_key_event(key(code, KeyEventKind::Press));
    assert!(screen.cell_on(36, 0));
    for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
        screen.handle_key_event(key(code, kind));
        assert!(screen.cell_on(36, 0));
    }
    screen.handle_key_event(key(code, KeyEventKind::Press));
    assert!(!screen.cell_on(36, 0));
    screen.handle_key_event(key(code, KeyEventKind::Release));
    assert!(!screen.cell_on(36, 0));
    screen.handle_key_event(key(KeyCode::Char('l'), KeyEventKind::Release));
    screen.handle_key_event(key(KeyCode::Right, KeyEventKind::Release));
    assert_eq!(screen.cursor_step(), 0);
}

#[test]
fn enter_toggles_then_moves_right_once_per_press_and_stays_at_the_right_edge() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".into(), Some(vec![36]), Vec::new(), Vec::new());
    screen.handle_key_event(key(KeyCode::Enter, KeyEventKind::Press));
    assert!(screen.cell_on(36, 0));
    assert_eq!(screen.cursor_step(), 1);
    for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
        screen.handle_key_event(key(KeyCode::Enter, kind));
    }
    assert_eq!(screen.cursor_step(), 1);
    assert!(!screen.cell_on(36, 1));

    screen.cursor_step = DRUM_STEPS - 1;
    screen.handle_key_event(key(KeyCode::Enter, KeyEventKind::Press));
    assert!(screen.cell_on(36, DRUM_STEPS - 1));
    assert_eq!(screen.cursor_step(), DRUM_STEPS - 1);
}

fn type_keys(screen: &mut DrumSequencerScreen, keys: &str) {
    for ch in keys.chars() {
        let code = if ch == '\n' {
            KeyCode::Enter
        } else {
            KeyCode::Char(ch)
        };
        screen.handle_key_event(key(code, KeyEventKind::Press));
    }
}

#[test]
fn count_prefix_repeats_the_next_key() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit(
        "full range".into(),
        Some((0..128).collect()),
        Vec::new(),
        Vec::new(),
    );

    // 2Enter は今のセルと右のセルを ON にし、その右へ進む。
    type_keys(&mut screen, "2\n");
    assert!(screen.cell_on(0, 0) && screen.cell_on(0, 1));
    assert!(!screen.cell_on(0, 2));
    assert_eq!(screen.cursor_step(), 2);
    // 右端で止まり、同じセルを OFF に戻さない。
    type_keys(&mut screen, "99\n");
    assert!((2..DRUM_STEPS).all(|step| screen.cell_on(0, step)));
    assert_eq!(screen.cursor_step(), DRUM_STEPS - 1);

    // 16k は 16 行上、10h は 10 step 左。回数は 1 回で消える。
    type_keys(&mut screen, "16k10hk");
    assert_eq!(screen.cursor_note(), Some(17));
    assert_eq!(screen.cursor_step(), 5);
    type_keys(&mut screen, "999k3j2l");
    assert_eq!(screen.cursor_note(), Some(124));
    assert_eq!(screen.cursor_step(), 7);

    // Space は回数を捨てて 1 回だけ切り替える。続く 12+ で 4 + 12 = 16 step。
    type_keys(&mut screen, "3 12+");
    assert_eq!(screen.cell_length(124, 7), Some(16));
    type_keys(&mut screen, "20-");
    assert_eq!(screen.cell_length(124, 7), Some(1));
    type_keys(&mut screen, "4,");
    assert_eq!(screen.cell_hit(124, 7).unwrap().velocity, 127 - 32);
    type_keys(&mut screen, "99.");
    assert_eq!(screen.cell_hit(124, 7).unwrap().velocity, 127);

    // 先頭の 0 は回数にならない。途中の 0 は桁になる。
    type_keys(&mut screen, "0l");
    assert_eq!(screen.cursor_step(), 8);
    type_keys(&mut screen, "10h");
    assert_eq!(screen.cursor_step(), 0);
    type_keys(&mut screen, "3]");
    assert_eq!(screen.pattern_index(), 3);
    type_keys(&mut screen, "5[");
    assert_eq!(screen.pattern_index(), PATTERN_COUNT - 2);

    // 数字の repeat と、host が消費したキーの後は回数にならない。
    type_keys(&mut screen, "2");
    screen.handle_key_event(key(KeyCode::Char('2'), KeyEventKind::Repeat));
    type_keys(&mut screen, "l");
    assert_eq!(screen.cursor_step(), 2);
    type_keys(&mut screen, "5");
    screen.clear_count();
    type_keys(&mut screen, "l");
    assert_eq!(screen.cursor_step(), 3);
    // host のキー（a など）も回数を消費する。
    type_keys(&mut screen, "5al");
    assert_eq!(screen.cursor_step(), 4);
}

#[test]
fn help_swallows_every_key_and_closes_only_with_question_or_escape() {
    for close in [KeyCode::Char('?'), KeyCode::Esc] {
        let mut screen = DrumSequencerScreen::default();
        screen.set_kit("kit".into(), Some(vec![36, 38]), Vec::new(), Vec::new());
        // kit が空でも help は開ける。ここでは kit ありで背面の不変を見る。
        assert!(screen.handle_key_event(key(KeyCode::Char('?'), KeyEventKind::Press)));
        assert!(screen.help_open());
        for code in [
            KeyCode::Char('?'),
            KeyCode::Char('l'),
            KeyCode::Up,
            KeyCode::Char(' '),
            KeyCode::Enter,
            KeyCode::Char('t'),
            KeyCode::Char('P'),
            KeyCode::Char('q'),
        ] {
            let kind = if code == KeyCode::Char('?') {
                KeyEventKind::Release
            } else {
                KeyEventKind::Press
            };
            assert!(screen.handle_key_event(key(code, kind)), "{code:?}");
        }
        assert!(screen.help_open());
        assert_eq!((screen.cursor_note(), screen.cursor_step()), (Some(36), 0));
        assert!(!screen.cell_on(36, 0));

        assert!(screen.handle_key_event(key(close, KeyEventKind::Press)));
        assert!(!screen.help_open(), "{close:?}");
        // 閉じたキーの Release は背面の編集にならない。
        screen.handle_key_event(key(close, KeyEventKind::Release));
        assert!(!screen.cell_on(36, 0));
    }
    let mut empty = DrumSequencerScreen::default();
    empty.handle_key_event(key(KeyCode::Char('?'), KeyEventKind::Press));
    assert!(empty.help_open());
}
