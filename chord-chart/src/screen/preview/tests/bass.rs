//! Bass preview ON/OFF の状態とキー境界。

use super::*;

fn uppercase_b(modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(KeyCode::Char('B'), modifiers)
}

#[test]
fn bass_preview_defaults_on_and_can_be_restored_without_requesting_playback() {
    let song = screen().song;
    for screen in [
        ChordChartScreen::default(),
        ChordChartScreen::new(song.clone()),
        ChordChartScreen::restored(Some(song)),
        ChordChartScreen::restored(None),
    ] {
        assert!(screen.bass_enabled());
    }

    let mut screen = entered();
    screen.set_bass_enabled(false);
    assert!(!screen.bass_enabled());
    assert_eq!(screen.take_preview(), None);

    screen.set_bass_enabled(true);
    assert!(screen.bass_enabled());
    assert_eq!(screen.take_preview(), None);
}

#[test]
fn plain_uppercase_b_toggles_and_restarts_the_current_section_without_changing_the_song() {
    for modifiers in [KeyModifiers::NONE, KeyModifiers::SHIFT] {
        let mut screen = entered();
        screen.handle_key_event(key(KeyCode::Char('j')));
        screen.take_preview();
        let song_before = screen.song.clone();

        assert_eq!(
            screen.handle_key_event(uppercase_b(modifiers)),
            ChordChartAction::PreviewSettingChanged
        );

        assert!(!screen.bass_enabled());
        assert_eq!(screen.song, song_before);
        assert_eq!(
            taken(&mut screen),
            PreviewRequest::section("B", "IIm-V-I-VIm", None)
        );
    }
}

#[test]
fn a_second_uppercase_b_turns_bass_back_on_and_requests_the_section_again() {
    let mut screen = entered();

    screen.handle_key_event(uppercase_b(KeyModifiers::SHIFT));
    screen.take_preview();
    assert!(!screen.bass_enabled());

    assert_eq!(
        screen.handle_key_event(uppercase_b(KeyModifiers::SHIFT)),
        ChordChartAction::PreviewSettingChanged
    );
    assert!(screen.bass_enabled());
    assert_eq!(taken(&mut screen).chord_index, None);
}

#[test]
fn lowercase_b_keeps_opening_key_and_bpm_input() {
    let mut screen = entered();

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('b'))),
        ChordChartAction::Continue
    );

    assert!(screen.line_input_open());
    assert!(screen.bass_enabled());
    assert_eq!(screen.take_preview(), None);
}

#[test]
fn control_or_alt_uppercase_b_is_ignored() {
    for modifiers in [
        KeyModifiers::CONTROL,
        KeyModifiers::ALT,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        KeyModifiers::ALT | KeyModifiers::SHIFT,
    ] {
        let mut screen = entered();

        assert_eq!(
            screen.handle_key_event(uppercase_b(modifiers)),
            ChordChartAction::Continue,
            "modifiers={modifiers:?}"
        );
        assert!(screen.bass_enabled(), "modifiers={modifiers:?}");
        assert_eq!(screen.take_preview(), None, "modifiers={modifiers:?}");
    }
}

#[test]
fn uppercase_b_is_swallowed_while_help_or_line_input_is_open() {
    let mut help = entered();
    help.handle_key_event(key(KeyCode::Char('?')));
    assert_eq!(
        help.handle_key_event(uppercase_b(KeyModifiers::SHIFT)),
        ChordChartAction::Continue
    );
    assert!(help.bass_enabled());
    assert_eq!(help.take_preview(), None);

    let mut input = entered();
    input.handle_key_event(key(KeyCode::Char('n')));
    assert_eq!(
        input.handle_key_event(uppercase_b(KeyModifiers::SHIFT)),
        ChordChartAction::Continue
    );
    assert!(input.bass_enabled());
    assert_eq!(input.take_preview(), None);
}

#[test]
fn uppercase_b_on_an_empty_screen_requests_silence() {
    let mut screen = ChordChartScreen::new(Song::empty());

    assert_eq!(
        screen.handle_key_event(uppercase_b(KeyModifiers::SHIFT)),
        ChordChartAction::PreviewSettingChanged
    );

    assert!(!screen.bass_enabled());
    assert_eq!(taken(&mut screen), PreviewRequest::silent());
}
