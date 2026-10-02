//! param pane の `j` / `k`（行）と `h` / `l`（値）。

use super::*;

#[test]
fn the_rows_show_down_only_for_up_down_and_tempo_only_for_a_chord() {
    use ArpRow::*;
    let mut screen = screen_with_overlay();
    assert_eq!(
        screen.arp_rows(),
        [Material, Pattern, Octaves, Shift, Humanize, Picking, Accent]
    );
    screen.set_arp(on(ArpPattern::UpDown));
    assert_eq!(
        screen.arp_rows(),
        [Material, Pattern, Octaves, Shift, Down, Humanize, Picking, Accent]
    );
    screen.select_arp_material("Am7");
    assert_eq!(screen.arp_rows(), ALL_ROWS);
}

#[test]
fn j_and_k_move_the_row_without_playing_and_stop_at_the_ends() {
    let mut screen = screen_with_overlay();
    assert_eq!(selected_row(&screen), ArpRow::Material);
    assert_eq!(
        screen.handle_key_event(char_key('k')),
        GuitarArticulationAction::Continue
    );
    assert_eq!(selected_row(&screen), ArpRow::Material);

    let mut walked = Vec::new();
    for _ in 0..7 {
        assert_eq!(
            screen.handle_key_event(char_key('j')),
            GuitarArticulationAction::Continue
        );
        walked.push(selected_row(&screen));
    }
    use ArpRow::*;
    assert_eq!(
        walked,
        [Pattern, Octaves, Shift, Humanize, Picking, Accent, Accent]
    );
    screen.handle_key_event(key(KeyCode::Up));
    assert_eq!(selected_row(&screen), Picking);
    screen.handle_key_event(key(KeyCode::Down));
    assert_eq!(selected_row(&screen), Accent);
}

#[test]
fn a_hidden_row_falls_back_to_the_row_before_it() {
    let mut screen = screen_with_mml("Am7");
    screen.handle_key_event(key(KeyCode::Char('z')));
    select_row(&mut screen, ArpRow::Rate);

    screen.select_arp_material("l16cdef");

    assert_eq!(selected_row(&screen), ArpRow::Shift);
    screen.handle_key_event(char_key('k'));
    assert_eq!(selected_row(&screen), ArpRow::Octaves);
}

#[test]
fn h_and_l_walk_the_patterns_and_stop_at_the_ends() {
    let mut screen = screen_with_overlay();
    select_row(&mut screen, ArpRow::Pattern);
    assert_eq!(*screen.arp(), on(ArpPattern::Up));

    assert_eq!(
        screen.handle_key_event(char_key('h')),
        GuitarArticulationAction::Continue
    );

    let mut walked = Vec::new();
    for _ in 0..4 {
        screen.handle_key_event(char_key('l'));
        walked.push(screen.arp().pattern);
    }
    use ArpPattern::*;
    assert_eq!(walked, [Down, UpDown, DownUp, DownUp]);
    screen.handle_key_event(key(KeyCode::Left));
    assert_eq!(screen.arp().pattern, UpDown);
    assert_eq!(
        screen.events(Take::Plain),
        crate::performance_events("l16cdef", screen.applied_arp())
            .unwrap()
            .as_slice()
    );
}

#[test]
fn the_octaves_stop_at_one_and_three() {
    let mut screen = screen_with_overlay();
    select_row(&mut screen, ArpRow::Octaves);
    let mut walked = Vec::new();
    for _ in 0..3 {
        screen.handle_key_event(char_key('l'));
        walked.push(screen.arp().octaves);
    }
    assert_eq!(walked, [2, 3, 3]);
    for _ in 0..3 {
        screen.handle_key_event(char_key('h'));
    }
    assert_eq!(screen.arp().octaves, 1);
    assert_eq!(
        screen.handle_key_event(char_key('h')),
        GuitarArticulationAction::Continue
    );
}

#[test]
fn the_down_walks_one_to_eight_and_all() {
    let mut screen = screen_with_overlay();
    screen.set_arp(on(ArpPattern::UpDown));
    select_row(&mut screen, ArpRow::Down);
    assert_eq!(screen.arp().down, None, "既定は全部");
    assert_eq!(
        screen.handle_key_event(char_key('l')),
        GuitarArticulationAction::Continue
    );

    let mut walked = Vec::new();
    for _ in 0..9 {
        screen.handle_key_event(char_key('h'));
        walked.push(screen.arp().down);
    }
    let mut expected: Vec<_> = (1..=8).rev().map(Some).collect();
    expected.push(Some(1));
    assert_eq!(walked, expected);
    // `l16cdef` の UpDown 1 周期 = 上り 4 + 下り 1。
    assert_eq!(screen.column_count(), 5);
}

#[test]
fn the_shift_moves_one_octave_and_stops_at_its_ends() {
    let mut screen = screen_with_overlay();
    screen.set_arp(on(ArpPattern::Up));
    select_row(&mut screen, ArpRow::Shift);
    let pitches = |screen: &GuitarArticulationScreen| -> Vec<u8> {
        crate::notes_from_events(screen.events(Take::Plain))
            .iter()
            .map(|note| note.pitch)
            .collect()
    };
    let base = pitches(&screen);

    assert_eq!(
        screen.handle_key_event(char_key('h')),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert_eq!(screen.arp().shift, -1);
    assert_eq!(
        pitches(&screen),
        base.iter().map(|pitch| pitch - 12).collect::<Vec<_>>()
    );
    screen.handle_key_event(char_key('h'));
    assert_eq!(
        screen.handle_key_event(char_key('h')),
        GuitarArticulationAction::Continue
    );
    assert_eq!(screen.arp().shift, -2);

    let mut walked = Vec::new();
    for _ in 0..5 {
        screen.handle_key_event(char_key('l'));
        walked.push(screen.arp().shift);
    }
    assert_eq!(walked, [-1, 0, 1, 2, 2]);
}
