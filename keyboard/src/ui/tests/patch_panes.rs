use super::*;
use cmrt_tui_core::theme::MONOKAI_FG;

/// 200 桁では keyboard pane が 74 桁で終わり、右の Patch selector の枠の内側に
/// Role / Preset / Patches が並ぶ。focus 中（初期は Patches）の枠だけが黄色。
#[test]
fn the_screen_draws_role_preset_and_patch_panes_inside_the_patch_selector_frame() {
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
        cmrt_patch_select::host_patch_catalog(&state),
        snapshot.role_presets(),
        Some("Basses/Sub Bass.fxp"),
    );
    let mut terminal = Terminal::new(TestBackend::new(200, 30)).unwrap();
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

    let rows: Vec<Vec<char>> = screen_text
        .lines()
        .take(2)
        .map(|line| line.chars().collect())
        .collect();
    let (top, inner_top) = (&rows[0], &rows[1]);
    assert_eq!(top[73], '┐', "{screen_text}");
    assert_eq!(top[74], '┌', "{screen_text}");
    assert!(
        top[74..]
            .iter()
            .collect::<String>()
            .starts_with("┌ Patch selector ─"),
        "{screen_text}"
    );
    assert_eq!(top[174], '┐', "{screen_text}");
    assert_eq!(top[175], '┌', "{screen_text}");
    assert_eq!(inner_top[74], '│', "{screen_text}");
    assert_eq!(inner_top[75], '┌', "{screen_text}");
    assert_eq!(inner_top[94], '┌', "{screen_text}");
    assert_eq!(inner_top[118], '┌', "{screen_text}");
    assert_eq!(inner_top[173], '┐', "{screen_text}");
    assert_eq!(inner_top[174], '│', "{screen_text}");

    let buffer = terminal.backend().buffer();
    let border_color = |x: u16, y: u16| buffer.cell((x, y)).unwrap().fg;
    assert_eq!(border_color(0, 0), MONOKAI_FG);
    assert_eq!(border_color(74, 0), MONOKAI_FG);
    assert_eq!(border_color(75, 1), MONOKAI_FG);
    assert_eq!(border_color(94, 1), MONOKAI_FG);
    assert_eq!(border_color(118, 1), MONOKAI_YELLOW);
}

struct NoVoicing;

impl crate::KeyboardVoicingLookup for NoVoicing {
    fn cached_voicing(&self, _patch: &str) -> Option<cmrt_realtime_play::PatchVoicing> {
        None
    }
}

fn render_screen(screen: &mut crate::KeyboardScreen<'_>) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(200, 30)).unwrap();
    terminal
        .draw(|f| {
            draw(
                screen,
                &crate::KeyboardConnectionStatus::default(),
                std::time::Instant::now(),
                f,
            )
        })
        .unwrap();
    terminal
}

/// 入力中は Patches pane の枠の内側の最下段に欄が重なり、不正な条件では枠が赤。
/// 確定後は欄が消え、pane の title に `/条件` が残る。
#[test]
fn the_filter_input_sits_at_the_bottom_of_the_patches_pane() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let names = ["Leads/Lead 1.fxp", "Leads/Lead 2.fxp", "Pads/Warm Pad.fxp"];
    let load = cmrt_tui_core::patch_load::PatchLoadState::ready(
        names
            .iter()
            .map(|name| (name.to_string(), name.to_lowercase()))
            .collect(),
    );
    let ctx = crate::KeyboardContext {
        patch_dirs_configured: true,
        patch_load: &load,
        voicing: &NoVoicing,
        catalog_notes: &[],
    };
    let mut screen = crate::KeyboardScreen::new(
        None,
        KeyboardState::new(Some("Leads/Lead 1.fxp".to_string())),
        crate::KeyboardMmlInput::default(),
        crate::KeyboardNoteGuide::new(None),
    );
    let press = |screen: &mut crate::KeyboardScreen<'_>, code| {
        screen.handle_key(KeyEvent::new(code, KeyModifiers::NONE), &ctx);
    };
    press(&mut screen, KeyCode::Char('/'));
    for ch in "lead".chars() {
        press(&mut screen, KeyCode::Char(ch));
    }

    // Patch selector の外枠は y=0..26（下 4 行は status と help）。その内側の Patches pane は
    // x=118..174・y=1..25 で、Patches の枠の内側の最下段 3 行は y=21..24。
    let terminal = render_screen(&mut screen);
    let text = buffer_to_string(&terminal);
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[21].contains(" /filter"), "{text}");
    assert!(lines[22].contains("lead"), "{text}");
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer.cell((118, 24)).unwrap().symbol(), "└", "{text}");
    assert_eq!(buffer.cell((119, 21)).unwrap().fg, MONOKAI_YELLOW);

    press(&mut screen, KeyCode::Char('('));
    let terminal = render_screen(&mut screen);
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer.cell((119, 21)).unwrap().fg,
        ratatui::style::Color::Red
    );

    press(&mut screen, KeyCode::Backspace);
    press(&mut screen, KeyCode::Enter);
    let text = buffer_to_string(&render_screen(&mut screen));
    assert!(text.contains(" Patches (1/2) /lead "), "{text}");
    assert!(!text.contains(" /filter"), "{text}");
}

/// `M` で開く overlay は patch selector と同じ title で、Patches pane の条件の solo/mute を印で出す。
#[test]
fn the_plugin_menu_overlay_marks_the_soloed_plugin() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let entries = [("Bell.syx/00", "Dexed"), ("Harp.floe-preset", "Floe")]
        .iter()
        .map(|(name, plugin)| {
            cmrt_patch_select::PatchCatalogEntry::new(
                name.to_string(),
                name.to_lowercase(),
                plugin.to_string(),
                None,
            )
        })
        .collect();
    let pairs = vec![(
        "Harp.floe-preset".to_string(),
        "harp.floe-preset".to_string(),
    )];
    let load = cmrt_tui_core::patch_load::PatchLoadState::ready(pairs);
    let ctx = crate::KeyboardContext {
        patch_dirs_configured: true,
        patch_load: &load,
        voicing: &NoVoicing,
        catalog_notes: &[],
    };
    let mut screen = crate::KeyboardScreen::new(
        None,
        KeyboardState::new(Some("Harp.floe-preset".to_string())),
        crate::KeyboardMmlInput::default(),
        crate::KeyboardNoteGuide::new(None),
    );
    screen.sync_patch_catalog(&ctx);
    screen.state.patch_catalog.load(
        cmrt_patch_select::HostPatchCatalog {
            catalog: cmrt_patch_select::PatchCatalogSnapshot::Ready(entries),
            patch_role_index: cmrt_patches::PatchRoleIndex::default(),
            load_measurements: Default::default(),
        },
        &[],
        Some("Harp.floe-preset"),
    );
    let press = |ch| KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
    screen.handle_key(press('M'), &ctx);
    screen.handle_key(press('f'), &ctx);
    screen.handle_key(press('M'), &ctx);

    let text = buffer_to_string(&render_screen(&mut screen));

    assert!(
        text.contains(" plugin solo/mute  a-z:solo  A-Z:mute"),
        "{text}"
    );
    let line = |needle: &str| {
        text.lines()
            .find(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("{needle}\n{text}"))
            .to_string()
    };
    assert!(line(" f  floe").contains("solo"), "{text}");
    assert!(!line(" d  dexed").contains("solo"), "{text}");
    assert!(text.contains("M:plugin solo/mute"), "{text}");
}
