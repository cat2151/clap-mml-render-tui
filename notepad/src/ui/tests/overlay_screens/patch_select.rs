use super::*;
use crate::tests::open_patch_select_for_test;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const PATCH_SELECT_OVERLAY_WIDTH_PERCENT: u16 = 88;
const PATCH_SELECT_OVERLAY_HEIGHT_PERCENT: u16 = 76;

const PAD_LINE: &str = r#"{"Surge XT patch":"Pads/Pad 1.fxp"} abc"#;

fn open_pads(app: &mut NotepadScreen<'_>) {
    open_patch_select_for_test(
        app,
        PAD_LINE,
        &["Pads/Pad 1.fxp", "Pads/Pad 2.fxp", "Leads/Lead 1.fxp"],
    );
}

fn press(app: &mut NotepadScreen<'_>, code: KeyCode) {
    app.handle_patch_select(KeyEvent::new(code, KeyModifiers::NONE));
}

#[test]
fn patch_select_screen_renders_the_shared_three_panes_over_the_normal_screen() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.favorite_patches = vec!["Pads/Pad 2.fxp".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 2.fxp".to_string(),
        PatchPhraseState {
            history: vec![],
            favorites: vec!["abc".to_string()],
        },
    );
    open_pads(&mut app);

    let lines = render_lines(&mut app, 120, 24).join("\n");
    let normalized = lines.replace(' ', "");

    assert!(lines.contains("[PATCH SELECT] notepad mode"), "{lines}");
    assert!(lines.contains("Role"), "{lines}");
    assert!(lines.contains("Preset"), "{lines}");
    assert!(lines.contains("★ Favorite"), "{lines}");
    assert!(lines.contains("Pads/Pad 1.fxp"), "{lines}");
    assert!(normalized.contains("現在1行目/全2行(1/2)"), "{lines}");
}

#[test]
fn patch_select_screen_marks_only_favorite_patches_in_the_star_column() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.favorite_patches = vec!["Pads/Pad 2.fxp".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 2.fxp".to_string(),
        PatchPhraseState {
            history: vec![],
            favorites: vec!["abc".to_string()],
        },
    );
    open_pads(&mut app);

    let lines = render_lines(&mut app, 120, 24);
    let row = |patch: &str| {
        lines
            .iter()
            .find(|line| line.contains(patch))
            .unwrap_or_else(|| panic!("{patch} row: {lines:#?}"))
            .clone()
    };

    assert!(row("Pads/Pad 2.fxp").contains('★'));
    assert!(!row("Pads/Pad 1.fxp").contains('★'));
}

#[test]
fn patch_select_screen_marks_memory_cached_preview_items() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_pads(&mut app);
    app.audio.cache.lock().unwrap().clear();
    app.audio.cache.lock().unwrap().insert(
        r#"{"Surge XT patch": "Pads/Pad 2.fxp"} abc"#.to_string(),
        vec![0.1, 0.2],
    );

    let lines = render_lines(&mut app, 120, 24);
    let row = |patch: &str| {
        lines
            .iter()
            .find(|line| line.contains(patch))
            .unwrap_or_else(|| panic!("{patch} row: {lines:#?}"))
            .clone()
    };

    assert!(row("Pads/Pad 2.fxp").contains('♪'));
    assert!(!row("Pads/Pad 1.fxp").contains('♪'));
}

#[test]
fn patch_select_screen_marks_silent_previews_in_dark_gray() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_pads(&mut app);
    {
        let mut cache = app.audio.cache.lock().unwrap();
        cache.clear();
        cache.insert(PAD_LINE.to_string(), vec![0.0, 0.1]);
        cache.insert(
            r#"{"Surge XT patch": "Pads/Pad 2.fxp"} abc"#.to_string(),
            vec![0.0; 4],
        );
    }

    let buffer = render_buffer(&mut app, 120, 24);
    let row_cells = |patch: &str| {
        let y = (0..buffer.area.height)
            .find(|&y| {
                (0..buffer.area.width)
                    .map(|x| buffer.cell((x, y)).unwrap().symbol())
                    .collect::<String>()
                    .contains(patch)
            })
            .unwrap_or_else(|| panic!("{patch} row"));
        (0..buffer.area.width)
            .map(|x| buffer.cell((x, y)).unwrap().clone())
            .collect::<Vec<_>>()
    };

    let silent = row_cells("Pads/Pad 2.fxp");
    let silent_mark = silent
        .iter()
        .find(|cell| cell.symbol() == "無")
        .expect("silent mark");
    assert_eq!(silent_mark.fg, cmrt_tui_core::theme::MONOKAI_DARK_GRAY);
    assert!(!silent.iter().any(|cell| cell.symbol() == "♪"));

    let sounding = row_cells("Pads/Pad 1.fxp");
    assert!(sounding.iter().any(|cell| cell.symbol() == "♪"));
    assert!(!sounding.iter().any(|cell| cell.symbol() == "無"));
}

#[test]
fn patch_select_screen_hides_the_play_settings_hint() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_pads(&mut app);

    let normalized = render_lines(&mut app, 160, 24).join("\n").replace(' ', "");

    assert!(normalized.contains("/:編集"), "{normalized}");
    assert!(!normalized.contains("S:演奏設定"), "{normalized}");
}

#[test]
fn patch_select_screen_shows_the_editing_title_and_moves_the_cursor_to_the_regex() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_pads(&mut app);
    press(&mut app, KeyCode::Char('/'));
    press(&mut app, KeyCode::Char('2'));

    let lines = render_lines(&mut app, 120, 24);
    let normalized = lines.join("\n").replace(' ', "");
    let cursor = render_cursor_position(&mut app, 120, 24);
    let regex_row = lines
        .iter()
        .position(|line| line.replace(' ', "").contains("Enter:絞り込み確定"))
        .expect("editing title row");

    assert!(normalized.contains("Regex(空白=AND)Enter:絞り込み確定"));
    assert_eq!(usize::from(cursor.y), regex_row + 1);
}

#[test]
fn patch_select_screen_shows_an_empty_list_when_nothing_matches() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_pads(&mut app);
    press(&mut app, KeyCode::Char('/'));
    for c in "zzz".chars() {
        press(&mut app, KeyCode::Char(c));
    }

    let normalized = render_lines(&mut app, 120, 24).join("\n").replace(' ', "");

    assert!(normalized.contains("音色(0/0/3)"), "{normalized}");
    assert!(normalized.contains("現在0/0"), "{normalized}");
}

#[test]
fn patch_select_overlay_uses_yellow_outer_border() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_pads(&mut app);

    let buffer = render_buffer(&mut app, 100, 24);
    let overlay_area = cmrt_tui_core::ui::centered_rect(
        PATCH_SELECT_OVERLAY_WIDTH_PERCENT,
        PATCH_SELECT_OVERLAY_HEIGHT_PERCENT,
        buffer.area,
    );
    let outer_border = buffer.cell((overlay_area.x, overlay_area.y)).unwrap();

    assert_eq!(outer_border.symbol(), "┌");
    assert_eq!(outer_border.fg, MONOKAI_YELLOW);
}

#[test]
fn patch_select_screen_splits_status_and_keybinds() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.test_set_render_job_status(
        r#"{"Surge XT patch":"Brass/Brass 1.fxp"} c"#,
        Some(crate::TuiRenderJobStatus::Running {
            elapsed: std::time::Duration::ZERO,
        }),
    );
    app.test_set_render_job_status(
        r#"{"Surge XT patch":"sfz/Guitar/04-Guitar.sfz"} c"#,
        Some(crate::TuiRenderJobStatus::Pending),
    );
    open_pads(&mut app);

    let lines = render_lines(&mut app, 200, 24);
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();
    let keybind_row = normalized_lines
        .iter()
        .position(|line| line.contains("/:Regex検索"))
        .unwrap();
    let render_row = keybind_row - 1;
    let status_row = render_row - 1;

    assert!(normalized_lines[status_row].contains("音色選択"));
    assert!(!normalized_lines[status_row].contains("sort:"));
    assert!(normalized_lines[render_row].contains("render実行:Brass1render順番待ち:04-Guitar"));
    assert!(!normalized_lines[keybind_row].contains("Ctrl+S"));
    assert!(normalized_lines[keybind_row].contains("n/p/t:overlay切替"));
    assert!(normalized_lines[keybind_row].contains("f:お気に入り"));
}

/// 設定不足でカタログから外れたプラグインの案内は、一覧が十分にあるときにも出る。
#[test]
fn patch_select_screen_shows_why_a_plugin_is_missing_from_the_catalog() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.catalog_notes = vec!["Vaporizer2 は patches_dirs が無いため一覧に出ません".to_string()];
    open_pads(&mut app);

    let lines = render_lines(&mut app, 120, 24).join("\n");

    assert!(lines.contains("Vaporizer2"), "{lines}");
    assert!(lines.contains("patches_dirs"), "{lines}");
}

/// 外れたプラグインが無ければ 1 文字も出さない。
#[test]
fn patch_select_screen_shows_no_catalog_note_when_nothing_was_skipped() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_pads(&mut app);

    let lines = render_lines(&mut app, 120, 24).join("\n");

    assert!(!lines.contains("Vaporizer2"), "{lines}");
}

#[test]
fn heavy_preview_confirm_dialog_is_drawn_over_patch_select() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_pads(&mut app);
    app.heavy_preview = Some(crate::input::HeavyPreview::Confirm {
        patch_name: "sfz/Guitar/04-Standard Guitar VSOP XTracking.sfz".to_string(),
        sample_bytes: 296_700_000,
    });

    let screen = render_lines(&mut app, 200, 30)
        .iter()
        .map(|line| line.replace(' ', ""))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        screen.contains("04-StandardGuitarVSOPXTracking"),
        "{screen}"
    );
    assert!(screen.contains("sample296MB"), "{screen}");
    assert!(screen.contains("y/Enter:試聴する"), "{screen}");
}
