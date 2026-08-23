//! `.vvp` の音色が**全画面へ同じ 1 本の一覧から届く**ことの番人。
//!
//! 各画面（notepad / keyboard / grid sequencer / MML 入力オーバーレイ / DAW）は
//! 自前で音色を走査せず、起動時に 1 回だけ作る共有の一覧
//! （`session::spawn_patch_loader` が埋める `patch_load_state`）を読む。
//! **画面ごとに `.vvp` 対応を入れて回る必要が無い根拠がここ**なので、
//! 「共有の一覧に入れたものがそのまま各画面へ出る」ことをテストで固定しておく。
//!
//! 一覧の中身そのもの（実プリセット 460 件が載るか）は `cmrt patch-roles --config` と
//! `cmrt-patches` の `#[ignore]` テストが別経路で数えている。

use super::*;

use crate::tui::ui::tests::render_lines;

/// 3 種別が混ざった一覧。`.vvp` にはアポストロフィ入りの実名も混ぜる
/// （MML の和音記法が `'...'` なので、素通しできているかを見る）。
fn mixed_patches() -> Vec<(String, String)> {
    make_patches(&[
        "patches_factory/Pads/Pad 1.fxp",
        "Dexed_01.syx/00 Say Again.",
        "AR Accent Arp.vvp",
        "AT I'll House Your Grains.vvp",
        "PD Emily.vvp",
    ])
}

/// **`patch_load_state` を作り直さずに中身だけ差し替える。** 作り直すと
/// notepad と共有している `Arc` が切れて、この一式が見たい「1 本の一覧」で
/// なくなってしまう。
fn app_with_mixed_patches() -> TuiApp<'static> {
    let app = TuiApp::new_for_test(test_config());
    *app.patch_load_state.lock().unwrap() = PatchLoadState::ready(mixed_patches());
    app
}

/// MML 入力オーバーレイ（Ctrl+P）が読む一覧に `.vvp` がそのまま出る。
#[test]
fn the_mml_overlay_sees_the_vvp_patches() {
    let app = app_with_mixed_patches();

    let pairs = app.loaded_patch_pairs();

    assert_eq!(pairs, mixed_patches());
    assert!(pairs
        .iter()
        .any(|(display, _)| display == "AT I'll House Your Grains.vvp"));
}

/// keyboard 画面へ `.vvp` の音色を持って入れる。
#[test]
fn the_keyboard_screen_starts_on_a_vvp_patch() {
    let mut app = app_with_mixed_patches();

    app.start_keyboard(Some("AR Accent Arp.vvp".to_string()));

    assert_eq!(
        app.active_screen,
        crate::screen_switch::PrimaryScreen::Keyboard
    );
    assert_eq!(app.keyboard.state.patch(), Some("AR Accent Arp.vvp"));
}

/// notepad の行に書いた `.vvp` を keyboard へ引き継げる。
/// **アポストロフィ入りの名前**でも先頭 JSON から読み戻せること。
#[test]
fn a_vvp_patch_with_an_apostrophe_survives_the_mml_head_json() {
    let mut app = app_with_mixed_patches();
    app.notepad.set_session_lines_for_test(vec![
        r#"{"Surge XT patch":"AT I'll House Your Grains.vvp"} c"#.to_string(),
    ]);

    app.start_keyboard_from_notepad();

    assert_eq!(
        app.keyboard.state.patch(),
        Some("AT I'll House Your Grains.vvp")
    );
}

/// 画面が読む一覧は **notepad と同じ実体**（`Arc` 共有）。
/// 別々に走査すると、片方だけ古い一覧を見る状態ができる。
#[test]
fn every_screen_reads_the_one_shared_patch_list() {
    let app = app_with_mixed_patches();

    assert!(Arc::ptr_eq(
        &app.patch_load_state,
        &app.notepad.patch_load_state
    ));
    // 共有できていれば、notepad 側から見ても同じ 5 件が入っている。
    let from_notepad = match &*app.notepad.patch_load_state.lock().unwrap() {
        PatchLoadState::Ready(snapshot) => snapshot.pairs().to_vec(),
        _ => panic!("notepad 側の一覧が共有されていない"),
    };
    assert_eq!(from_notepad, mixed_patches());
}

/// 「設定不足でカタログから外れたプラグインがある」ことの案内も、
/// 一覧と同じく **1 か所で数えて全画面へ配る**。
///
/// 画面ごとに数えると、案内が出ない画面がひとつ残っても誰も気づけない
/// （**案内が無いこと自体が症状**なので、見ただけでは分からない）。
/// 数えるのは `cmrt_runtime::catalog_notice_lines`、文言は
/// `SkippedCatalogPlugin::notice_line` が単一ソース。
#[test]
fn the_screens_show_which_plugins_are_missing_from_the_catalog() {
    let note = "Vaporizer2 は patches_dirs が無いため一覧に出ません";
    let mut app = app_with_mixed_patches();
    app.catalog_notes = vec![note.to_string()];

    // MML 入力オーバーレイ（Ctrl+P → Ctrl+T）。
    app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    app.handle_mml_overlay_key_event(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
    let overlay = render_lines(&mut app, 100, 24).join("\n");
    assert!(overlay.contains("Vaporizer2"), "{overlay}");

    // keyboard 画面（音色一覧が常時出ている画面）。
    app.handle_mml_overlay_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.handle_mml_overlay_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.start_keyboard(None);
    let keyboard = render_lines(&mut app, 100, 24).join("\n");
    assert!(keyboard.contains("Vaporizer2"), "{keyboard}");
}

/// 外れたプラグインが無ければ、どの画面にも案内は出ない。
/// **ふだんの見え方を 1 文字も変えない**ことの番人。
#[test]
fn the_screens_show_no_note_when_every_plugin_is_in_the_catalog() {
    let mut app = app_with_mixed_patches();
    assert!(app.catalog_notes.is_empty());

    app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    app.handle_mml_overlay_key_event(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
    let overlay = render_lines(&mut app, 100, 24).join("\n");

    assert!(!overlay.contains("Vaporizer2"), "{overlay}");
}
