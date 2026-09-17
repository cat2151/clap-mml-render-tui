use super::*;
use cmrt_tui_core::theme::MONOKAI_CYAN;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn render_note_grid(screen: &GridSequencerScreen) -> String {
    let mut terminal = Terminal::new(TestBackend::new(90, 24)).unwrap();
    let connection = screen.connection_status();
    terminal
        .draw(|frame| {
            let area = frame.area();
            crate::ui::grid::draw(screen, &connection, frame, area);
        })
        .unwrap();
    buffer_to_string(&terminal)
}

#[test]
fn the_patch_selector_overlay_shows_the_three_panes_and_the_patches() {
    let patches = vec![
        ("Keys/Alpha.fxp".to_string(), "keys/alpha.fxp".to_string()),
        ("Keys/Beta.fxp".to_string(), "keys/beta.fxp".to_string()),
    ];
    let patch_load = cmrt_tui_core::patch_load::PatchLoadState::ready(patches);
    let ctx = crate::tests::ctx_with(
        &patch_load,
        crate::tests::empty_catalog(),
        &crate::NoVoicingLookup,
    );
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());
    screen.open_patch_selector(0, &ctx);

    let rendered = render(&screen);

    assert!(rendered.contains("instance 1 patch select"), "{rendered}");
    assert!(rendered.contains(" Role "), "{rendered}");
    assert!(rendered.contains(" Preset "), "{rendered}");
    assert!(rendered.contains("Patches (1/2/2)"), "{rendered}");
    // 音色名は grid の PATCH 欄にも出る。selector の行だけが `▶ ` を持つ。
    let patch_lines = |patch: &str| {
        rendered
            .lines()
            .filter(|line| line.contains(patch))
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    assert!(
        patch_lines("Keys/Alpha.fxp")
            .iter()
            .any(|line| line.contains("▶ ")),
        "{rendered}"
    );
    assert!(
        patch_lines("Keys/Beta.fxp")
            .iter()
            .all(|line| !line.contains('▶')),
        "{rendered}"
    );
    assert!(rendered.contains("h/l:pane"), "{rendered}");
    assert!(rendered.contains("r:random"), "{rendered}");
    assert!(rendered.contains("/:regex"), "{rendered}");
    assert!(rendered.contains("click/Enter:apply"), "{rendered}");
    assert!(!rendered.contains(" Regex "), "{rendered}");
}

#[test]
fn the_regex_field_narrows_the_patches_and_counts_them_in_the_title() {
    let patches = vec![
        (
            "Guitars/Soft Strum.fxp".to_string(),
            "guitars/soft strum.fxp".to_string(),
        ),
        (
            "Sequences/Hard Strum.fxp".to_string(),
            "sequences/hard strum.fxp".to_string(),
        ),
        (
            "Strum/Plain Pad.fxp".to_string(),
            "strum/plain pad.fxp".to_string(),
        ),
    ];
    let patch_load = cmrt_tui_core::patch_load::PatchLoadState::ready(patches);
    let ctx = crate::tests::ctx_with(
        &patch_load,
        crate::tests::empty_catalog(),
        &crate::NoVoicingLookup,
    );
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);
    screen.handle_patch_selector_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE), &ctx);
    for ch in "strum".chars() {
        screen
            .handle_patch_selector_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE), &ctx);
    }

    let terminal = terminal_for(&screen);
    let rendered = buffer_to_string(&terminal);

    assert!(rendered.contains(" Regex "), "{rendered}");
    find_text_ignoring_spaces(terminal.backend().buffer(), "strum");
    assert!(rendered.contains("Patches (1/3/3)"), "{rendered}");
    assert!(rendered.contains("Guitars/Soft Strum.fxp"), "{rendered}");
    assert!(rendered.contains("Strum/Plain Pad.fxp"), "{rendered}");
    assert!(rendered.contains("Enter:confirm"), "{rendered}");
}

#[test]
fn an_empty_regex_field_is_shown_while_editing_and_keeps_the_whole_list() {
    let patches = vec![
        ("Keys/Alpha.fxp".to_string(), "keys/alpha.fxp".to_string()),
        ("Keys/Beta.fxp".to_string(), "keys/beta.fxp".to_string()),
    ];
    let patch_load = cmrt_tui_core::patch_load::PatchLoadState::ready(patches);
    let ctx = crate::tests::ctx_with(
        &patch_load,
        crate::tests::empty_catalog(),
        &crate::NoVoicingLookup,
    );
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);
    screen.handle_patch_selector_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE), &ctx);

    let rendered = render(&screen);

    assert!(rendered.contains(" Regex "), "{rendered}");
    assert!(rendered.contains("Patches (1/2/2)"), "{rendered}");
}

#[test]
fn the_grid_patch_name_follows_keyboard_and_random_previews_without_committing() {
    let patches = vec![
        ("Keys/Alpha.fxp".to_string(), "keys/alpha.fxp".to_string()),
        ("Keys/Beta.fxp".to_string(), "keys/beta.fxp".to_string()),
    ];
    let patch_load = cmrt_tui_core::patch_load::PatchLoadState::ready(patches);
    let ctx = crate::tests::ctx_with(
        &patch_load,
        crate::tests::empty_catalog(),
        &crate::NoVoicingLookup,
    );
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());
    screen.open_patch_selector(0, &ctx);

    screen.handle_patch_selector_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &ctx);
    assert!(render_note_grid(&screen).contains("Keys/Beta.fxp"));
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp"),
        "preview 中はまだ commit しない"
    );

    screen.handle_patch_selector_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE), &ctx);
    assert!(render_note_grid(&screen).contains("Keys/Alpha.fxp"));
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp"),
        "r preview でも state は未確定のまま"
    );
}

#[test]
fn a_manual_patch_load_greys_only_the_target_row() {
    let screen = GridSequencerScreen::with_track_count(None, 2);
    let connection = GridConnectionStatus {
        phase: GridConnectionPhase::Ready,
        row_patch: Some(crate::GridRowPatchStatus {
            row: 1,
            phase: crate::GridRowPatchPhase::Loading,
        }),
        ..GridConnectionStatus::default()
    };

    let terminal = terminal_with_connection(&screen, &connection);
    let rendered = render_with_connection(&screen, &connection);

    assert_eq!(row_label_fg(&terminal, &screen, 0), MONOKAI_FG);
    assert_eq!(row_label_fg(&terminal, &screen, 1), MONOKAI_GRAY);
    assert!(rendered.contains("instance 2 patch loading"), "{rendered}");
    assert!(!has_progress_overlay(&rendered), "{rendered}");
}

/// builtin preset の語を含む名前の一覧。行用途ごとの Role / Preset を見る用。
fn role_patch_load() -> cmrt_tui_core::patch_load::PatchLoadState {
    cmrt_tui_core::patch_load::PatchLoadState::ready(
        [
            "Basses/Bass 01.fxp",
            "Basses/Bass 02.fxp",
            "Drums/Hi Hat 01.wav",
            "Drums/Kick 01.wav",
            "Drums/Perc Clap 01.wav",
            "Drums/Snare 01.wav",
            "Leads/Lead 01.fxp",
            "Pads/Pad 01.fxp",
        ]
        .into_iter()
        .map(|patch| (patch.to_string(), patch.to_lowercase()))
        .collect(),
    )
}

/// chord mode ON・7 track。行 0〜2 が chord / bass / arpeggio、行 3〜6 が drum。
fn chord_mode_screen() -> GridSequencerScreen {
    let mut screen = GridSequencerScreen::with_track_count(None, 7);
    screen.state.set_chord(
        crate::ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );
    screen
}

fn drum_row(screen: &GridSequencerScreen, role: cmrt_rhythm::DrumRole) -> usize {
    (0..screen.state.instance_count())
        .find(|row| screen.state.drum_role(*row) == Some(role))
        .expect("7 track には全 drum 行がある")
}

/// popup 内の 3 pane が全部見える大きさで描く。
fn wide_terminal(screen: &GridSequencerScreen) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(200, 50)).unwrap();
    let connection = screen.connection_status();
    terminal.draw(|f| draw(screen, &connection, f)).unwrap();
    terminal
}

/// pane の左上角（枠線）の前景色。
fn corner_fg(terminal: &Terminal<TestBackend>, pane: Rect) -> Color {
    terminal
        .backend()
        .buffer()
        .cell((pane.x, pane.y))
        .unwrap()
        .fg
}

#[test]
fn the_bass_row_opens_with_the_patch_table_header_and_the_bass_role_selected() {
    let patch_load = role_patch_load();
    let ctx = crate::tests::ctx_with(
        &patch_load,
        crate::tests::empty_catalog(),
        &crate::NoVoicingLookup,
    );
    let mut screen = chord_mode_screen();
    screen.open_patch_selector(crate::BASS_ROW, &ctx);

    let rendered = buffer_to_string(&wide_terminal(&screen));

    assert!(rendered.contains(" Role "), "{rendered}");
    assert!(rendered.contains(" Preset "), "{rendered}");
    assert!(rendered.contains(" Patches ("), "{rendered}");
    let header = rendered
        .lines()
        .find(|line| line.contains("Category"))
        .unwrap_or_else(|| panic!("header is drawn: {rendered}"));
    assert!(header.contains("Patch"), "{header}");
    assert!(header.contains("Load"), "{header}");
    assert!(rendered.contains("▶ Bass track"), "{rendered}");
    assert!(rendered.contains("▶ ALL"), "{rendered}");
    assert!(!rendered.contains(" Regex "), "{rendered}");

    screen.handle_patch_selector_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE), &ctx);
    let rendered = buffer_to_string(&wide_terminal(&screen));
    assert!(rendered.contains(" Regex "), "{rendered}");
}

#[test]
fn the_kick_row_opens_in_the_drum_role_at_the_kick_preset() {
    let patch_load = role_patch_load();
    let ctx = crate::tests::ctx_with(
        &patch_load,
        crate::tests::empty_catalog(),
        &crate::NoVoicingLookup,
    );
    let mut screen = chord_mode_screen();
    screen.open_patch_selector(drum_row(&screen, cmrt_rhythm::DrumRole::Kick), &ctx);

    let rendered = buffer_to_string(&wide_terminal(&screen));

    assert!(rendered.contains("▶ Drum tracks"), "{rendered}");
    assert!(rendered.contains("▶ kick|bass drum"), "{rendered}");
}

#[test]
fn only_the_focused_pane_has_a_yellow_frame() {
    let patch_load = role_patch_load();
    let ctx = crate::tests::ctx_with(
        &patch_load,
        crate::tests::empty_catalog(),
        &crate::NoVoicingLookup,
    );
    let mut screen = chord_mode_screen();
    screen.open_patch_selector(crate::BASS_ROW, &ctx);
    let layout = crate::patch_selector::PatchSelectorLayout::new(Rect::new(0, 0, 200, 50), false);

    let terminal = wide_terminal(&screen);
    assert_eq!(corner_fg(&terminal, layout.role_pane), MONOKAI_CYAN);
    assert_eq!(corner_fg(&terminal, layout.preset_pane), MONOKAI_CYAN);
    assert_eq!(corner_fg(&terminal, layout.patch_pane), MONOKAI_YELLOW);

    screen.handle_patch_selector_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE), &ctx);
    screen.handle_patch_selector_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE), &ctx);
    let terminal = wide_terminal(&screen);
    assert_eq!(corner_fg(&terminal, layout.role_pane), MONOKAI_YELLOW);
    assert_eq!(corner_fg(&terminal, layout.preset_pane), MONOKAI_CYAN);
    assert_eq!(corner_fg(&terminal, layout.patch_pane), MONOKAI_CYAN);
}
