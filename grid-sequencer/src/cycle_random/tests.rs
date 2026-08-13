use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::GridSequencerScreen;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn screen() -> GridSequencerScreen {
    GridSequencerScreen::with_track_count(None, 4)
}

#[test]
fn the_default_is_everything_random() {
    assert_eq!(CycleRandom::default(), CycleRandom::ALL);
    assert!(CycleRandom::ALL.instances_change());
    assert!(!CycleRandom::NONE.instances_change());
    // 進行とテンポは instance を触らない。
    assert!(!CycleRandom::HOLD.instances_change());
}

#[test]
fn the_compact_label_shows_only_the_items_that_are_on() {
    assert_eq!(CycleRandom::ALL.compact_label(), "PNDACBS");
    assert_eq!(CycleRandom::NONE.compact_label(), "-------");
    assert_eq!(CycleRandom::HOLD.compact_label(), "----CB-");
    assert_eq!(
        CycleRandom {
            note: false,
            arp: false,
            ..CycleRandom::ALL
        }
        .compact_label(),
        "P-D-CBS"
    );
}

#[test]
fn every_item_round_trips_through_get_and_set() {
    for item in CycleRandomItem::ALL {
        let mut random = CycleRandom::ALL;
        random.set(item, false);
        assert!(!random.get(item), "{} が落ちていない", item.label());
        // 落としたのは1項目だけ。
        assert_eq!(
            CycleRandomItem::ALL
                .iter()
                .filter(|other| !random.get(**other))
                .count(),
            1
        );
    }
}

#[test]
fn the_number_keys_toggle_the_matching_item() {
    let mut screen = screen();
    screen.toggle_cycle_random_overlay();
    assert!(screen.cycle_random_open());

    for (index, item) in CycleRandomItem::ALL.iter().enumerate() {
        let digit = char::from_digit(index as u32 + 1, 10).expect("1..=6");
        screen.handle_cycle_random_key(press(KeyCode::Char(digit)));
        assert!(!screen.cycle_random().get(*item), "{}", item.label());
        assert_eq!(screen.cycle_random_cursor(), Some(index));
    }
    assert_eq!(screen.cycle_random(), CycleRandom::NONE);
}

#[test]
fn space_toggles_the_row_under_the_cursor() {
    let mut screen = screen();
    screen.toggle_cycle_random_overlay();

    screen.handle_cycle_random_key(press(KeyCode::Down));
    screen.handle_cycle_random_key(press(KeyCode::Char(' ')));

    assert_eq!(screen.cycle_random_cursor(), Some(1));
    assert!(!screen.cycle_random().note, "2番目は NOTE");
    assert!(screen.cycle_random().patch);
}

/// 上下端で止める。巡回させると端まで来たことが分からない。
#[test]
fn the_cursor_stops_at_both_ends() {
    let mut screen = screen();
    screen.toggle_cycle_random_overlay();

    screen.handle_cycle_random_key(press(KeyCode::Up));
    assert_eq!(screen.cycle_random_cursor(), Some(0));

    for _ in 0..CycleRandomItem::ALL.len() + 2 {
        screen.handle_cycle_random_key(press(KeyCode::Down));
    }
    assert_eq!(
        screen.cycle_random_cursor(),
        Some(CycleRandomItem::ALL.len() - 1)
    );
}

#[test]
fn shift_a_turns_everything_on_and_shift_n_turns_everything_off() {
    let mut screen = screen();
    screen.toggle_cycle_random_overlay();

    screen.handle_cycle_random_key(KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT));
    assert_eq!(screen.cycle_random(), CycleRandom::NONE);

    screen.handle_cycle_random_key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT));
    assert_eq!(screen.cycle_random(), CycleRandom::ALL);
}

#[test]
fn esc_q_and_a_all_close_the_overlay() {
    for code in [KeyCode::Esc, KeyCode::Char('q'), KeyCode::Char('a')] {
        let mut screen = screen();
        screen.toggle_cycle_random_overlay();
        screen.handle_cycle_random_key(press(code));
        assert!(!screen.cycle_random_open(), "{code:?} で閉じる");
    }
}

/// 手編集は触った項目だけを落とす。束ねて落としていた頃は、1セル描くだけで
/// 音色の引き直しまで止まっていた。
#[test]
fn a_manual_edit_only_stops_the_item_it_touched() {
    let mut screen = screen();

    screen.begin_manual_edit(CycleRandomItem::Note);

    assert!(!screen.cycle_random().note);
    assert!(screen.cycle_random().patch);
    assert!(screen.cycle_random().drum);
    assert!(screen.cycle_random().arp);
    assert!(screen.cycle_random().chord);
    assert!(screen.cycle_random().bpm);
}
