use super::*;

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
        normalized_lines
            .iter()
            .any(|line| line.contains("e:config.toml編集→再起動")),
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
fn daily_help_and_footer_do_not_show_project_file_actions() {
    let mut app = build_test_app();
    app.workspace_kind = crate::WorkspaceKind::Daily;
    app.daily_page_date = Some("2026-08-25".to_string());

    let normal_screen = render_lines(&app, 180, 52).join("\n");
    assert!(
        !normal_screen.contains("f:file"),
        "screen:\n{normal_screen}"
    );

    app.mode = DawMode::Help;
    let help_screen = render_lines(&app, 180, 52).join("\n");
    assert!(
        !help_screen.contains("project file"),
        "screen:\n{help_screen}"
    );
    assert!(
        !help_screen.contains("Open Daily Archive"),
        "screen:\n{help_screen}"
    );
    assert!(!help_screen.contains("Save As"), "screen:\n{help_screen}");
}

#[test]
fn history_help_draws_on_top_of_history_overlay() {
    let mut app = build_test_app();
    app.mode = DawMode::Help;
    app.help_origin = DawMode::History;
    app.overlays.history.patch_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
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

/// メモリ行は overlay の先頭に出す。ヘルプが端末より長いと下が切り落とされるため。
#[test]
fn help_shows_the_memory_usage_at_the_top() {
    let mut app = build_test_app();
    app.mode = DawMode::Help;

    let buffer = render_buffer(&app, 160, 52);
    let (_, top, _, _) = help_overlay_bounds(&buffer);
    let memory_line = render_lines(&app, 160, 52)[usize::from(top) + 1].replace(' ', "");

    assert!(memory_line.contains("実メモリ合計"), "{memory_line}");
    assert!(memory_line.contains("OS空き"), "{memory_line}");
}

#[test]
fn help_overlay_size_follows_daw_help_content() {
    let mut normal = build_test_app();
    normal.mode = DawMode::Help;

    let mut patch_select = build_test_app();
    patch_select.mode = DawMode::Help;
    patch_select.help_origin = DawMode::PatchSelect;
    patch_select.overlays.patch_select.all =
        vec![("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string())];
    patch_select.overlays.patch_select.filtered = vec!["Pads/Pad 1.fxp".to_string()];

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
    // 先頭に差し込むメモリ行 + 区切りの空行ぶんだけ、どのヘルプも 2 行高い。
    assert!(patch_height < 22, "patch={patch_height}");
    assert_ne!(
        normal_width, patch_width,
        "normal={normal_width} patch={patch_width}"
    );
    assert!(
        normal_height > patch_height,
        "normal={normal_height} patch={patch_height}"
    );
}
