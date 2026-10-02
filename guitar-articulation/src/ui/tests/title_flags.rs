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
fn the_title_shows_the_arp_only_while_the_overlay_is_open() {
    use cmrt_arpeggiator::ArpPattern;

    let mut screen = screen_with_mml("l16cdef");
    assert!(!top(&screen).contains("[arp:"), "{}", top(&screen));
    screen.handle_key_event(key(KeyCode::Char('z')));
    assert!(top(&screen).contains("[arp:Upx1]"), "{}", top(&screen));

    let short_down = crate::ArpSettings {
        pattern: ArpPattern::UpDown,
        octaves: 2,
        down: Some(2),
        ..crate::ArpSettings::default()
    };
    screen.set_arp(short_down);
    assert_eq!(
        crate::ui::arp_flag(Some(&short_down), false).as_deref(),
        Some(" [arp:UpDown x2 b2]")
    );
    assert!(
        top(&screen).contains("[arp:UpDownx2b2]"),
        "{}",
        top(&screen)
    );

    screen.set_arp(crate::ArpSettings {
        down: None,
        ..short_down
    });
    assert!(top(&screen).contains("[arp:UpDownx2]"), "{}", top(&screen));

    let lowered = crate::ArpSettings {
        shift: -1,
        ..short_down
    };
    assert_eq!(
        crate::ui::arp_flag(Some(&lowered), false).as_deref(),
        Some(" [arp:UpDown x2 o-1 b2]")
    );
    assert_eq!(
        crate::ui::arp_flag(
            Some(&crate::ArpSettings {
                shift: 2,
                ..short_down
            }),
            false
        )
        .as_deref(),
        Some(" [arp:UpDown x2 o+2 b2]")
    );
    screen.set_arp(lowered);
    assert!(
        top(&screen).contains("[arp:UpDownx2o-1b2]"),
        "{}",
        top(&screen)
    );

    screen.set_arp(crate::ArpSettings {
        pattern: ArpPattern::DownUp,
        ..short_down
    });
    assert!(
        top(&screen).contains("[arp:DownUpx2]"),
        "下り幅は UpDown のときだけ: {}",
        top(&screen)
    );

    screen.handle_key_event(key(KeyCode::Esc));
    assert!(!top(&screen).contains("[arp:"), "{}", top(&screen));
}

#[test]
fn the_title_shows_the_bpm_and_rate_only_for_a_chord_material() {
    use cmrt_arpeggiator::ArpPattern;

    let arp = crate::ArpSettings {
        pattern: ArpPattern::Up,
        bpm: 90,
        rate: crate::ArpRate::EighthTriplet,
        ..crate::ArpSettings::default()
    };
    assert_eq!(
        crate::ui::arp_flag(Some(&arp), true).as_deref(),
        Some(" [arp:Up x1 90bpm/8t]")
    );
    assert_eq!(
        crate::ui::arp_flag(Some(&arp), false).as_deref(),
        Some(" [arp:Up x1]")
    );

    let mut chord = screen_with_mml("Am7");
    chord.handle_key_event(key(KeyCode::Char('z')));
    chord.set_arp(arp);
    assert!(
        top(&chord).contains("[arp:Upx190bpm/8t]"),
        "{}",
        top(&chord)
    );
    let mut mml = screen_with_mml("l16cdef");
    mml.handle_key_event(key(KeyCode::Char('z')));
    mml.set_arp(arp);
    assert!(top(&mml).contains("[arp:Upx1]"), "{}", top(&mml));
}
