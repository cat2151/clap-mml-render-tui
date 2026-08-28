//! MML 入力オーバーレイ（`Ctrl+P`）は grid の上へ重ねて出す。
//!
//! 全角文字は buffer 上でセルごとに分かれるので、assert は ASCII 部分で書くこと。

use super::*;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn open_overlay_on_a_playable_cell(app: &mut DawApp) {
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] = r#"{"Surge XT patch": "Bass/Overlay Bass.fxp"}"#.to_string();
    app.editor.data[2][1] = "l8cdefg".to_string();
    assert!(app.open_mml_overlay());
}

#[test]
fn draw_shows_the_input_box_with_the_cell_mml_and_the_track_patch() {
    let mut app = build_test_app();
    open_overlay_on_a_playable_cell(&mut app);

    let screen = render_lines(&app, 100, 24).join("\n");

    assert!(
        screen.contains("l8cdefg"),
        "そのセルの MML が入力欄に入っているはず:\n{screen}"
    );
    assert!(
        screen.contains("Bass/Overlay Bass.fxp"),
        "枠のタイトルへ、そのセルが実際に鳴る音色を出すはず:\n{screen}"
    );
    assert!(
        screen.contains('┌') && screen.contains('┘'),
        "枠付きで重ねて出すはず:\n{screen}"
    );
    assert!(app.uses_textarea_cursor());
}

#[test]
fn the_footer_switches_to_the_overlay_keys() {
    let mut app = build_test_app();
    let normal_footer = normalized_screen(&app, 240, 24);
    assert!(
        normal_footer.contains("i/Ctrl+P:MML"),
        "NORMAL の footer は `i` がオーバーレイの入口だと伝えるはず:\n{normal_footer}"
    );
    assert!(!normal_footer.contains("i:INS"));

    open_overlay_on_a_playable_cell(&mut app);
    let overlay_footer = normalized_screen(&app, 240, 24);

    assert!(
        overlay_footer.contains("Ctrl+T:") && overlay_footer.contains("Ctrl+O:"),
        "オーバーレイのキー割り当てを footer に出すはず:\n{overlay_footer}"
    );
    assert!(!overlay_footer.contains("i:INS"));
}

/// 確定キーの案内を footer に出す。`Enter` が「次の小節へ」だと分かるのが要点で、
/// これが無いと 1 行モードの `Enter` が改行に見える。
#[test]
fn the_overlay_footer_explains_the_two_commit_keys() {
    let mut app = build_test_app();
    open_overlay_on_a_playable_cell(&mut app);

    let footer = normalized_screen(&app, 240, 24);

    assert!(
        footer.contains("Enter:確定→次小節"),
        "Enter が次小節へ進む確定だと footer に出すはず:\n{footer}"
    );
    assert!(
        footer.contains("ESC:確定→閉じる"),
        "ESC も確定だと footer に出すはず:\n{footer}"
    );
}

/// `?` の help は NORMAL から開く。オーバーレイの操作もそこへ載せる。
#[test]
fn the_help_page_lists_the_overlay_keys() {
    let mut app = build_test_app();
    app.mode = DawMode::Help;

    let help = normalized_screen(&app, 160, 60);

    assert!(
        help.contains("i/Ctrl+P:MML入力オーバーレイ"),
        "NORMAL の項に `i` の新しい役目を出すはず:\n{help}"
    );
    assert!(
        help.contains("Enter:確定→次小節の入力欄を開く"),
        "オーバーレイの Enter を help に出すはず:\n{help}"
    );
    assert!(
        help.contains("Ctrl+T:音色選択"),
        "オーバーレイの Ctrl+T を help に出すはず:\n{help}"
    );
    assert!(
        !help.contains("i:INSERTモード"),
        "`i` が INSERT だという古い案内は残らないはず:\n{help}"
    );
}

/// ratatui のテスト描画では全角文字がセルごとに分かれるので、空白を落としてから比べる。
fn normalized_screen(app: &DawApp, width: u16, height: u16) -> String {
    render_lines(app, width, height)
        .into_iter()
        .map(|line| line.replace(' ', ""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn ctrl_o_lists_the_phrase_history_of_the_track_patch() {
    let mut app = build_test_app();
    app.patch_phrase_store.patches.insert(
        "Bass/Overlay Bass.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec!["o2l16ccdd".to_string()],
            favorites: vec!["o3l4gg".to_string()],
        },
    );
    open_overlay_on_a_playable_cell(&mut app);

    app.handle_mml_overlay_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL));
    let screen = render_lines(&app, 120, 30).join("\n");

    assert!(
        screen.contains("o2l16ccdd"),
        "その音色のフレーズ履歴を渡しているはず:\n{screen}"
    );
    assert!(
        screen.contains("o3l4gg"),
        "お気に入りも渡しているはず:\n{screen}"
    );
}

#[test]
fn ctrl_t_lists_the_injected_catalog_patches() {
    let mut app = build_test_app();
    *app.patch_load.lock().unwrap() = cmrt_tui_core::patch_load::PatchLoadState::ready(vec![
        (
            "Bass/Overlay Bass.fxp".to_string(),
            "bass/overlay bass.fxp".to_string(),
        ),
        (
            "Pads/Overlay Pad.fxp".to_string(),
            "pads/overlay pad.fxp".to_string(),
        ),
    ]);
    open_overlay_on_a_playable_cell(&mut app);

    app.handle_mml_overlay_key_event(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
    let screen = render_lines(&app, 120, 30).join("\n");

    assert!(
        screen.contains("Pads/Overlay Pad.fxp"),
        "注入 snapshot の音色が一覧に出るはず:\n{screen}"
    );
}

/// オーバーレイで音色を確定すると、grid の init 列（Stage 4 の `role:音色名`）が変わる。
/// 「そのセルが実際に鳴る音色」と表示が一致していることを、描画バッファで確かめる。
#[test]
fn confirming_a_patch_updates_the_init_column_of_the_grid() {
    let mut app = build_test_app();
    *app.patch_load.lock().unwrap() = cmrt_tui_core::patch_load::PatchLoadState::ready(vec![
        (
            "Bass/Overlay Bass.fxp".to_string(),
            "bass/overlay bass.fxp".to_string(),
        ),
        (
            "Leads/Overlay Lead.fxp".to_string(),
            "leads/overlay lead.fxp".to_string(),
        ),
    ]);
    open_overlay_on_a_playable_cell(&mut app);
    let before = render_lines(&app, 120, 30).join("\n");
    assert!(
        before.contains("bass:Overlay"),
        "開いた時点では元の音色が出ているはず:\n{before}"
    );

    app.handle_mml_overlay_key_event(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
    for ch in "overlay lead".chars() {
        app.handle_mml_overlay_key_event(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
    }
    app.handle_mml_overlay_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_mml_overlay_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    let after = render_lines(&app, 120, 30).join("\n");

    assert!(
        after.contains("lead:Overlay"),
        "確定した音色が init 列へ出るはず:\n{after}"
    );
    assert!(
        !after.contains("bass:Overlay"),
        "元の音色は残らないはず:\n{after}"
    );
}
