//! 見出しの [repeat]・[accent:]・[arp:] が、画面の状態に従う。

use super::*;

fn top(screen: &GuitarArticulationScreen) -> String {
    squeezed(&rows_in(&render(screen), Rect::new(0, 0, WIDTH, 1)))
}

fn shift(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT)
}

#[test]
fn the_title_shows_repeat_while_it_is_on() {
    let mut screen = screen_with_mml("l16cdef");
    assert!(!top(&screen).contains("[repeat]"));
    screen.handle_key_event(shift('R'));
    assert!(top(&screen).contains("[repeat]"), "{}", top(&screen));
    screen.handle_key_event(shift('R'));
    assert!(!top(&screen).contains("[repeat]"));
}

#[test]
fn the_title_shows_the_accent_pattern_only_while_dynamics_are_on() {
    let mut screen = screen_with_mml("l16cdef");
    assert!(!top(&screen).contains("[accent:"), "{}", top(&screen));

    screen.handle_key_event(key(KeyCode::Char('d')));
    assert!(top(&screen).contains("[accent:上]"), "{}", top(&screen));
    screen.handle_key_event(shift('A'));
    assert!(top(&screen).contains("[accent:下]"), "{}", top(&screen));
    screen.handle_key_event(shift('A'));
    assert!(top(&screen).contains("[accent:上下]"), "{}", top(&screen));
}

#[test]
fn the_title_shows_auto2_and_its_accent_pattern_only_in_on2() {
    let mut screen = screen_with_mml("l16cdef");
    screen.handle_key_event(key(KeyCode::Char('s')));
    assert!(!top(&screen).contains("[auto2]"), "{}", top(&screen));
    assert!(!top(&screen).contains("[accent:"), "{}", top(&screen));

    screen.handle_key_event(key(KeyCode::Char('s')));
    assert!(
        top(&screen).contains("[auto2][accent:上]"),
        "{}",
        top(&screen)
    );

    screen.handle_key_event(key(KeyCode::Char('s')));
    assert!(!top(&screen).contains("[auto2]"), "{}", top(&screen));
}

#[test]
fn the_title_shows_the_arp_only_while_it_is_on() {
    use cmrt_arpeggiator::ArpPattern;

    let mut screen = screen_with_mml("l16cdef");
    assert!(!top(&screen).contains("[arp:"), "{}", top(&screen));

    let up_turn = crate::ArpSettings {
        enabled: true,
        pattern: ArpPattern::UpTurn,
        octaves: 2,
        ..crate::ArpSettings::default()
    };
    screen.set_arp(up_turn);
    assert_eq!(
        crate::ui::arp_flag(&up_turn).as_deref(),
        Some(" [arp:UpTurn x2 N2 b2]")
    );
    assert!(
        top(&screen).contains("[arp:UpTurnx2N2b2]"),
        "{}",
        top(&screen)
    );

    screen.set_arp(crate::ArpSettings {
        pattern: ArpPattern::UpDown,
        ..up_turn
    });
    assert!(
        top(&screen).contains("[arp:UpDownx2N2]"),
        "{}",
        top(&screen)
    );

    screen.set_arp(crate::ArpSettings {
        enabled: false,
        ..up_turn
    });
    assert!(!top(&screen).contains("[arp:"), "{}", top(&screen));
}
