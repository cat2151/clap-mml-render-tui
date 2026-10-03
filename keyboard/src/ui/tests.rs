use super::*;
use crate::{
    guide::{KeyboardNoteGuidePresentation, KEYBOARD_NOTE_GUIDE_MESSAGE},
    KeyboardState, KEYBOARD_NOTES,
};
use cmrt_tui_core::theme::MONOKAI_YELLOW;
use ratatui::{backend::TestBackend, style::Modifier, Terminal};

mod controller;
mod effect_pane;
mod note_columns;
mod note_mode_bar;
mod patch_panes;
mod playback_status;
mod share_notice;

fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_numeric_overlay(state: &KeyboardState) -> String {
    let backend = TestBackend::new(80, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| draw_numeric_input_overlay(state.numeric_input(), state.cc_number(), f, f.area()))
        .unwrap();
    buffer_to_string(&terminal)
}

fn render_mml_overlay(input: &crate::KeyboardMmlInput<'_>) -> String {
    let backend = TestBackend::new(80, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| draw_mml_input_overlay(input, f, f.area()))
        .unwrap();
    buffer_to_string(&terminal)
}

fn render_note_guide(presentation: KeyboardNoteGuidePresentation) -> Terminal<TestBackend> {
    let backend = TestBackend::new(80, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| draw_note_guide_overlay(presentation, f, f.area()))
        .unwrap();
    terminal
}

fn render_keyboard_help(presentation: KeyboardNoteGuidePresentation) -> Terminal<TestBackend> {
    let backend = TestBackend::new(100, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            f.render_widget(
                Paragraph::new(keyboard_help_lines(presentation, None, false)).style(base_style()),
                f.area(),
            );
        })
        .unwrap();
    terminal
}

fn has_colored_message_start(terminal: &Terminal<TestBackend>) -> bool {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height).any(|y| {
        (0..buffer.area.width).any(|x| {
            let cell = buffer.cell((x, y)).unwrap();
            cell.symbol() == "c"
                && cell.fg == MONOKAI_YELLOW
                && cell.modifier.contains(Modifier::BOLD)
        })
    })
}

fn status_text(state: &KeyboardState) -> String {
    note_playback_status_line(state, std::time::Instant::now())
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

#[test]
fn the_playback_status_label_follows_the_effective_mode() {
    let mut state = KeyboardState::default();
    assert_eq!(status_text(&state), "Chord: -");

    assert!(state.press(KEYBOARD_NOTES[4]).is_some());
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert!(state.press(KEYBOARD_NOTES[2]).is_some());
    let now = std::time::Instant::now();
    assert_eq!(status_text(&state), "Chord: G4 C4 E4");
    let _ = state.cycle_note_playback(now);
    assert_eq!(state.note_playback_mode(), crate::NotePlaybackMode::Auto);
    assert_eq!(status_text(&state), "Repeat: G4 C4 E4");
    let _ = state.cycle_note_playback(now);
    assert_eq!(status_text(&state), "Repeat: G4 C4 E4");
    let _ = state.cycle_note_playback(now);
    assert_eq!(state.note_playback_mode(), crate::NotePlaybackMode::Arp);
    assert_eq!(status_text(&state), "Arp: C4 E4 G4 C5 E5 G5");
}

#[test]
fn auto_on_a_mono_patch_is_labeled_arp() {
    let mut state = KeyboardState::default();
    state.set_detected_voicing(cmrt_realtime_play::PatchVoicing::Mono);
    state.replace_repeat_chords(vec![vec![67, 60]], std::time::Instant::now(), false);
    let _ = state.cycle_note_playback(std::time::Instant::now());

    assert_eq!(state.note_playback_mode(), crate::NotePlaybackMode::Auto);
    assert_eq!(status_text(&state), "Arp: C4 G4 C5 G5");
}

#[test]
fn note_playback_status_formats_arbitrary_midi_notes() {
    let mut state = KeyboardState::default();
    state.replace_repeat_chords(
        vec![vec![0, 61], vec![127]],
        std::time::Instant::now(),
        false,
    );

    assert_eq!(status_text(&state), "Chord: C-1 C#4 | G9");
}

#[test]
fn note_playback_status_shows_each_arp_chord_as_its_arp_sequence_in_progression_order() {
    let mut state = KeyboardState::default();
    let now = std::time::Instant::now();
    state.replace_repeat_chords(vec![vec![67, 60], vec![69, 65]], now, false);
    for _ in 0..3 {
        let _ = state.cycle_note_playback(now);
    }

    assert_eq!(status_text(&state), "Arp: C4 G4 C5 G5 | F4 A4 F5 A5");
}

#[test]
fn send_duration_uses_microseconds_then_milliseconds() {
    assert_eq!(
        format_send_duration(std::time::Duration::from_micros(42)),
        "42 us"
    );
    assert_eq!(
        format_send_duration(std::time::Duration::from_micros(12_345)),
        "12.3 ms"
    );
}

#[test]
fn voicing_status_keeps_probe_and_surge_disagreement_visible() {
    let report: cmrt_realtime_play::VoicingReport = serde_json::from_value(serde_json::json!({
        "decision": "poly",
        "probe": {"result": "mono", "ended_note_ids": [1], "blocks": 1},
        "voice_info": null,
        "surge": {
            "scene_mode": "Dual",
            "active_scene": "A",
            "scene_a_play_mode": "Mono",
            "scene_b_play_mode": "Poly",
            "result": "mixed"
        },
        "disagreement": true
    }))
    .unwrap();

    assert_eq!(
        voicing_status_text(&KeyboardVoicingStatus::Detected(report.clone())),
        "detect: poly [probe:mono Surge:mixed !]"
    );
    assert_eq!(
        voicing_status_text(&KeyboardVoicingStatus::Detecting {
            previous: Some(report)
        }),
        "detect: poly [probe:mono Surge:mixed !] (probing new patch)"
    );
}

#[test]
fn cached_voicing_is_labeled_as_cached() {
    assert_eq!(
        voicing_status_text(&KeyboardVoicingStatus::Cached(
            cmrt_realtime_play::PatchVoicing::Mono
        )),
        "detect: mono (cached)"
    );
}

#[test]
fn cc_number_input_overlay_shows_typed_digits_and_key_help() {
    let mut state = KeyboardState::default();
    state.begin_numeric_input(NumericInputTarget::CcNumber);
    state.numeric_input_push('7');
    state.numeric_input_push('4');

    let screen = render_numeric_overlay(&state);
    // 全角文字はセル埋めの空白を挟んで描画されるため、空白を除去して比較する
    let normalized = screen.replace(' ', "");
    assert!(screen.contains("CC number"));
    assert!(normalized.contains("CC番号を入力:74_"));
    assert!(normalized.contains("Enter:確定"));
}

#[test]
fn cc_value_input_overlay_names_the_target_cc_number() {
    let mut state = KeyboardState::default();
    state.begin_numeric_input(NumericInputTarget::CcValue);

    let screen = render_numeric_overlay(&state);
    let normalized = screen.replace(' ', "");
    assert!(screen.contains("CC value"));
    assert!(normalized.contains("CC値を入力(CC#1へ送信):_"));
}

#[test]
fn no_numeric_input_draws_no_overlay() {
    assert!(render_numeric_overlay(&KeyboardState::default())
        .chars()
        .all(char::is_whitespace));
}

#[test]
fn mml_input_overlay_shows_text_and_key_help() {
    let mut input = crate::KeyboardMmlInput::default();
    input.open();
    for ch in "o4ceg".chars() {
        input.input(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char(ch),
            crossterm::event::KeyModifiers::NONE,
        ));
    }

    let screen = render_mml_overlay(&input);
    assert!(screen.contains("MML notes"));
    assert!(screen.contains("o4ceg"));
    assert!(screen.replace(' ', "").contains("Enter:確定"));
}

#[test]
fn mml_input_overlay_shows_conversion_error() {
    let mut input = crate::KeyboardMmlInput::default();
    input.open();
    input.input(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('r'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(input.confirm().is_none());

    let screen = render_mml_overlay(&input);
    assert!(screen
        .replace(' ', "")
        .contains("MMLに発音ノートがありません"));
}

#[test]
fn daily_note_guide_overlay_is_centered_and_colored() {
    let terminal = render_note_guide(KeyboardNoteGuidePresentation::Overlay);
    let screen = buffer_to_string(&terminal);

    assert!(screen.replace(' ', "").contains("音出し確認"));
    assert!(screen
        .replace(' ', "")
        .contains(KEYBOARD_NOTE_GUIDE_MESSAGE));
    assert!(has_colored_message_start(&terminal));
}

#[test]
fn hidden_and_footer_presentations_do_not_draw_note_guide_overlay() {
    for presentation in [
        KeyboardNoteGuidePresentation::Hidden,
        KeyboardNoteGuidePresentation::Footer,
    ] {
        assert!(buffer_to_string(&render_note_guide(presentation))
            .chars()
            .all(char::is_whitespace));
    }
}

#[test]
fn same_day_footer_replaces_normal_key_guide_with_colored_message() {
    let terminal = render_keyboard_help(KeyboardNoteGuidePresentation::Footer);
    let screen = buffer_to_string(&terminal);

    assert!(screen
        .replace(' ', "")
        .contains(KEYBOARD_NOTE_GUIDE_MESSAGE));
    assert!(!screen.contains("cdefgab:notes"));
    assert!(!screen.contains("h/l/Left/Right:pane"));
    assert!(has_colored_message_start(&terminal));
}

/// 設定不足でカタログから外れたプラグインの案内は、help 行の上に全幅で出る。
///
/// keyboard 画面の音色一覧は常時表示なので、開く操作を待たずに出す。
/// 一覧に**出てこない**ものの話なので、一覧をいくら眺めても分からない。
#[test]
fn the_screen_shows_why_a_plugin_is_missing_from_the_catalog() {
    let render = |notes: &[String]| {
        let mut screen = crate::KeyboardScreen::new(
            None,
            KeyboardState::default(),
            crate::KeyboardMmlInput::default(),
            crate::KeyboardNoteGuide::new(None),
        );
        screen.state.patch_catalog.set_catalog_notes(notes);
        let mut terminal = Terminal::new(TestBackend::new(90, 16)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    &mut screen,
                    &crate::KeyboardConnectionStatus::default(),
                    std::time::Instant::now(),
                    f,
                )
            })
            .unwrap();
        buffer_to_string(&terminal)
    };

    let with = render(&["Vaporizer2 は patches_dirs が無いため一覧に出ません".to_string()]);
    assert!(with.contains("Vaporizer2"), "{with}");
    assert!(with.contains("patches_dirs"), "{with}");

    // 案内が無ければ 1 行も増やさない。ふだんの見え方を変えないことの番人。
    let without = render(&[]);
    assert!(!without.contains("Vaporizer2"), "{without}");
}
