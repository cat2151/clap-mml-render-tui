use super::*;
use cmrt_tui_core::theme::MONOKAI_CYAN;

/// 200 桁では keyboard pane が 74 桁で終わり、右に Role / Preset / Patches が並ぶ。
/// focus 中（初期は Patches）の枠だけが黄色。
#[test]
fn the_screen_draws_role_preset_and_patch_panes_next_to_the_keyboard() {
    let pairs = [
        "Basses/Sub Bass.fxp",
        "Pads/Warm Pad.fxp",
        "Leads/Saw Lead.fxp",
    ]
    .iter()
    .map(|name| (name.to_string(), name.to_lowercase()))
    .collect();
    let state = cmrt_tui_core::patch_load::PatchLoadState::ready(pairs);
    let cmrt_tui_core::patch_load::PatchLoadState::Ready(snapshot) = &state else {
        unreachable!()
    };
    let mut screen = crate::KeyboardScreen::new(
        None,
        KeyboardState::new(Some("Basses/Sub Bass.fxp".to_string())),
        crate::KeyboardMmlInput::default(),
        crate::KeyboardNoteGuide::new(None),
    );
    screen.state.patch_catalog.load(
        cmrt_mml_overlay::host_patch_catalog(&state),
        snapshot.role_presets(),
        Some("Basses/Sub Bass.fxp"),
    );
    let mut terminal = Terminal::new(TestBackend::new(200, 30)).unwrap();
    terminal
        .draw(|f| draw(&mut screen, &crate::KeyboardConnectionStatus::default(), f))
        .unwrap();
    let screen_text = buffer_to_string(&terminal);

    for expected in [
        " Role (1/7)",
        " Preset (1/",
        " Patches (1/3)",
        "Category",
        "Patch",
        "Load",
        "Bass track",
        "Chord track",
        "Basses/Sub Bass.fxp",
    ] {
        assert!(screen_text.contains(expected), "{expected}\n{screen_text}");
    }

    let top: Vec<char> = screen_text.lines().next().unwrap().chars().collect();
    assert_eq!(top[73], '┐', "{screen_text}");
    assert_eq!(top[74], '┌', "{screen_text}");
    assert_eq!(top[96], '┌', "{screen_text}");
    assert_eq!(top[126], '┌', "{screen_text}");

    let buffer = terminal.backend().buffer();
    let border_color = |x: u16| buffer.cell((x, 0)).unwrap().fg;
    assert_eq!(border_color(0), MONOKAI_CYAN);
    assert_eq!(border_color(74), MONOKAI_CYAN);
    assert_eq!(border_color(96), MONOKAI_CYAN);
    assert_eq!(border_color(126), MONOKAI_YELLOW);
}
