use super::*;

#[test]
fn draw_shows_log_pane_with_all_borders() {
    let app = build_test_app();

    let lines = render_lines(&app, 60, 10);

    assert!(
        lines.iter().any(|line| line.contains("┌ log ")),
        "lines: {:?}",
        lines
    );
    assert!(
        lines.iter().any(|line| line.contains("└")),
        "lines: {:?}",
        lines
    );
}

#[test]
fn draw_shows_outer_border_in_monokai_cyan() {
    let app = build_test_app();

    let buffer = render_buffer(&app, 60, 10);

    assert_eq!(buffer.cell((0, 0)).unwrap().symbol(), "┌");
    assert_eq!(buffer.cell((59, 0)).unwrap().symbol(), "┐");
    assert_eq!(buffer.cell((0, 9)).unwrap().symbol(), "└");
    assert_eq!(buffer.cell((59, 9)).unwrap().symbol(), "┘");
    assert_eq!(buffer.cell((0, 0)).unwrap().fg, Color::Rgb(102, 217, 239));
}

#[test]
fn draw_shows_solo_and_mute_below_init_meas_during_solo_mode() {
    let mut app = build_test_app();
    app.solo_tracks[1] = true;

    let lines = render_lines(&app, 60, 20);

    assert!(
        lines.iter().any(|line| line.contains("solo")),
        "lines: {:?}",
        lines
    );
    assert!(
        lines.iter().any(|line| line.contains("mute")),
        "lines: {:?}",
        lines
    );
}

#[test]
fn draw_grays_out_muted_tracks_during_solo_mode() {
    let mut app = build_test_app();
    app.data[2][1] = "gabc".to_string();
    app.solo_tracks[1] = true;

    let buffer = render_buffer(&app, 60, 20);

    assert_eq!(buffer.cell((1, 6)).unwrap().fg, MONOKAI_GRAY);
    assert_eq!(buffer.cell((11, 6)).unwrap().fg, MONOKAI_GRAY);
}

#[test]
fn help_does_not_show_old_semicolon_guidance() {
    let mut app = build_test_app();
    app.mode = DawMode::Help;

    // ratatui のテスト描画では全角文字の間に空白が入るため、空白を除去して比較する。
    let normalized_lines: Vec<String> = render_lines(&app, 160, 52)
        .into_iter()
        .map(|line| line.replace(' ', ""))
        .collect();

    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("ヘルプ(Keybinds)")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("K/?:ヘルプ(このページ)")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("Ctrl+C/X/V:コピー/カット/ペースト")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("s:solotoggle")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("m:mixeroverlay")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("Shift+H:historyoverlay")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("dd:現在セルをyankして空にする")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("p:yank内容で現在セルを上書き")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("g:現在track/measにgenerateを反映してpreview")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("Shift+P:演奏/停止")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("Shift+Space:非play時、現在measから演奏開始して継続")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("a:off→start固定/end追従→end固定→off")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("n:notepadへ切替")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        !normalized_lines
            .iter()
            .any(|line| line.contains("スペース区切りでAND条件(例:basssoft)")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        !normalized_lines
            .iter()
            .any(|line| line.contains("Enter:(検索中)絞り込み入力を確定して操作に戻る")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        !normalized_lines
            .iter()
            .any(|line| line.contains("Enter:(通常)現在track/measに反映")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        !normalized_lines
            .iter()
            .any(|line| line.contains("Ctrl+C:強制終了")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        !normalized_lines
            .iter()
            .any(|line| line.contains("分割して下のtrackに追加")),
        "lines: {:?}",
        normalized_lines
    );
}

#[test]
fn history_help_draws_on_top_of_history_overlay() {
    let mut app = build_test_app();
    app.mode = DawMode::Help;
    app.help_origin = DawMode::History;
    app.history_overlay_patch_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );

    let normalized_lines: Vec<String> = render_lines(&app, 100, 52)
        .into_iter()
        .map(|line| line.replace(' ', ""))
        .collect();

    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("patchhistory-Pads/Pad1.fxp")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("┌ヘルプ(Keybinds)")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("HISTORYoverlay")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("?:ヘルプ(このページ)")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("スペース区切りでAND条件(例:basssoft)")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("Enter:(検索中)絞り込み入力を確定して操作に戻る")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("n:globalhistoryへ切り替え")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("p:current/selectedpatchhistoryへ切り替え")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("t:patchselectoverlayへ切り替え")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("Enter:(通常)現在track/measに反映")),
        "lines: {:?}",
        normalized_lines
    );
}

#[test]
fn help_overlay_size_follows_daw_help_content() {
    let mut normal = build_test_app();
    normal.mode = DawMode::Help;

    let mut patch_select = build_test_app();
    patch_select.mode = DawMode::Help;
    patch_select.help_origin = DawMode::PatchSelect;
    patch_select.patch_all = vec![("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string())];
    patch_select.patch_filtered = vec!["Pads/Pad 1.fxp".to_string()];

    let normal_buffer = render_buffer(&normal, 200, 60);
    let patch_select_buffer = render_buffer(&patch_select, 200, 60);
    let (normal_left, normal_top, normal_right, normal_bottom) =
        help_overlay_bounds(&normal_buffer);
    let (patch_left, patch_top, patch_right, patch_bottom) =
        help_overlay_bounds(&patch_select_buffer);

    let normal_width = normal_right - normal_left + 1;
    let normal_height = normal_bottom - normal_top + 1;
    let patch_width = patch_right - patch_left + 1;
    let patch_height = patch_bottom - patch_top + 1;

    assert!(
        patch_left > 0 && patch_top > 0,
        "bounds: {:?}",
        (patch_left, patch_top, patch_right, patch_bottom)
    );
    assert!(
        patch_right + 1 < patch_select_buffer.area.width,
        "bounds: {:?}",
        (patch_left, patch_top, patch_right, patch_bottom)
    );
    assert!(
        patch_bottom + 1 < patch_select_buffer.area.height,
        "bounds: {:?}",
        (patch_left, patch_top, patch_right, patch_bottom)
    );
    assert!(patch_width < 120, "patch={patch_width}");
    assert!(patch_height < 20, "patch={patch_height}");
    assert_ne!(
        normal_width, patch_width,
        "normal={normal_width} patch={patch_width}"
    );
    assert!(
        normal_height > patch_height,
        "normal={normal_height} patch={patch_height}"
    );
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

#[test]
fn draw_shows_mixer_overlay_with_track_labels_and_db_values() {
    let mut app = build_test_app();
    app.mode = DawMode::Mixer;
    app.mixer_cursor_track = 1;
    app.track_volumes_db[1] = -3;
    app.track_volumes_db[2] = 6;

    let normalized_lines: Vec<String> = render_lines(&app, 100, 30)
        .into_iter()
        .map(|line| line.to_lowercase())
        .collect();

    assert!(
        normalized_lines.iter().any(|line| line.contains("mixer")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("track1") && line.contains("track2")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("-3db") && line.contains("+6db")),
        "lines: {:?}",
        normalized_lines
    );
}

#[test]
fn draw_highlights_selected_mixer_track_with_contrast_background_without_blink() {
    let mut app = build_test_app();
    app.mode = DawMode::Mixer;
    app.mixer_cursor_track = 1;

    let buffer = render_buffer(&app, 100, 30);
    let highlighted_positions: Vec<(u16, u16)> = (0..100)
        .flat_map(|x| (0..30).map(move |y| (x, y)))
        .filter(|(x, y)| {
            let cell = buffer.cell((*x, *y)).unwrap();
            cell.bg == cursor_highlight_bg(cell.fg)
                && !cell
                    .modifier
                    .contains(ratatui::style::Modifier::RAPID_BLINK)
        })
        .collect();

    assert!(
        !highlighted_positions.is_empty(),
        "selected mixer track should use a contrast background"
    );

    let (x, y) = find_text_ignoring_spaces(&buffer, "track1");
    let cell = buffer.cell((x, y)).unwrap();
    assert_eq!(cell.fg, MONOKAI_FG);
    assert_eq!(cell.bg, cursor_highlight_bg(MONOKAI_FG));
}
