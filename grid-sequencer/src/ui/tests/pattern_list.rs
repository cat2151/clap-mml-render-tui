//! step セルの wheel が送るフレーズ型 list の pane。

use super::*;

fn chord_screen() -> GridSequencerScreen {
    let mut screen = screen_with_first_row(60, &[0, 3, 7]);
    screen.state.set_chord(
        crate::ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );
    screen
}

/// 2列paneが入る端末では grid の右へ全listが並ぶ。送り先が見えていることが目的なので、
/// 型の名前がそのまま読める形で出す。
#[test]
fn the_phrase_list_is_drawn_to_the_right_of_the_grid() {
    let rendered = render(&chord_screen());

    assert!(rendered.contains("Phrase"), "{rendered}");
    for label in [
        "Up",
        "UpDownHold",
        "Random",
        "Whole",
        "#-##+Oct",
        "####+Oct",
    ] {
        assert!(rendered.contains(label), "{label} が無い: {rendered}");
    }
}

/// list を出しても 16 step ぶんのセルは1つも削らない。
#[test]
fn the_pane_does_not_eat_into_the_sixteen_step_cells() {
    let screen = chord_screen();
    let rendered = render(&screen);
    let first_row = rendered.lines().nth(FIRST_ROW_Y + 1).unwrap();

    assert_eq!(
        slice_chars(first_row, first_cell_x(&screen), CELLS_WIDTH)
            .chars()
            .count(),
        CELLS_WIDTH
    );
}

/// chord mode off ではアルペジオ・ベースラインの list を出さない。
/// 出るのは drum の list だけ（drum は chord mode の外の機能）。
#[test]
fn the_arp_and_bass_lists_are_hidden_while_the_chord_mode_is_off() {
    let rendered = render(&GridSequencerScreen::new(None));

    assert!(!rendered.contains("UpDownHold"), "{rendered}");
    assert!(!rendered.contains("####+Oct"), "{rendered}");
    assert!(rendered.contains("drum kick"), "{rendered}");
}

/// drum 行も chord mode も無い構成では pane ごと出さない。
#[test]
fn the_pane_is_hidden_without_any_section() {
    let rendered = render(&GridSequencerScreen::with_track_count(None, 3));

    assert!(!rendered.contains("Phrase"), "{rendered}");
}

/// 直近に送った型に印が付く。どこまで送ったかが list 上で分かる。
#[test]
fn the_pattern_the_wheel_last_applied_is_marked_in_the_list() {
    let mut screen = chord_screen();
    screen
        .state
        .display_drawn_now(crate::DrawnPhrases::with_arp(crate::ArpPattern::UpDown));

    let rendered = render(&screen);

    assert!(
        rendered.lines().any(|line| line.contains("> UpDown")),
        "{rendered}"
    );
    assert!(
        rendered.lines().any(|line| line.contains("  Random")),
        "{rendered}"
    );
}
