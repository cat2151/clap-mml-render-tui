use super::*;

#[test]
fn draw_keeps_footer_on_last_row_when_idle() {
    let app = build_test_app();

    let lines = render_lines(&app, 120, 10);
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();

    let play_row = lines.len() - 5;
    let info_row = lines.len() - 4;
    let render_row = lines.len() - 3;
    let footer_row = lines.len() - 2;

    assert!(!lines[play_row].contains('▶'), "lines: {:?}", lines);
    assert!(
        !lines[info_row].contains("loop meas :"),
        "lines: {:?}",
        lines
    );
    assert!(
        normalized_lines[render_row].contains("並列render中:0"),
        "lines: {:?}",
        lines
    );
    assert!(lines[footer_row].contains("DAW"), "lines: {:?}", lines);
}

#[test]
fn draw_keeps_footer_color_cyan_across_play_states() {
    for play_state in [
        DawPlayState::Idle,
        DawPlayState::Playing,
        DawPlayState::Preview,
    ] {
        let app = build_test_app();
        {
            let mut state = app.playback.play_state.lock().unwrap();
            *state = play_state;
        }

        let buffer = render_buffer(&app, 120, 10);

        assert_eq!(
            buffer.cell((1, 8)).unwrap().fg,
            MONOKAI_CYAN,
            "footer color should stay cyan"
        );
    }
}

#[test]
fn normal_footer_shows_shift_h_history_shortcut() {
    let app = build_test_app();

    let normalized_lines: Vec<String> = render_lines(&app, FOOTER_FULL_KEYBIND_TEST_WIDTH, 20)
        .into_iter()
        .map(|line| line.replace(' ', ""))
        .collect();

    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("Shift+H:history")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines.iter().any(|line| line.contains("dd:cut")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("g:generate")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines.iter().any(|line| line.contains("p:paste")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines.iter().any(|line| line.contains("u:undo")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("Shift+P:play/stop")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("Shift+Space:fromhere")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("h/←・l/→:meas")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("n:notepad")),
        "lines: {:?}",
        normalized_lines
    );
}

/// chord 行は `j` / `k` から遠いので、往復キーがフッタから見つかること。
#[test]
fn normal_footer_shows_the_chord_row_jump_shortcut() {
    let app = build_test_app();

    let normalized_lines: Vec<String> = render_lines(&app, FOOTER_FULL_KEYBIND_TEST_WIDTH, 20)
        .into_iter()
        .map(|line| line.replace(' ', ""))
        .collect();

    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("C:chord行")),
        "lines: {:?}",
        normalized_lines
    );
}

/// `G` は破壊的（chord 行と init セルを書き換える）ので、案内がフッタに出ていること。
#[test]
fn normal_footer_shows_the_chord_wizard_shortcut() {
    let app = build_test_app();

    let normalized_lines: Vec<String> = render_lines(&app, FOOTER_FULL_KEYBIND_TEST_WIDTH, 20)
        .into_iter()
        .map(|line| line.replace(' ', ""))
        .collect();

    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("G:chordwizard")),
        "lines: {:?}",
        normalized_lines
    );
}
