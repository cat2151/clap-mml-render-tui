//! オーバーレイで選んだ音色を、その track の init セルへ反映する経路の検証。
//!
//! DAW にとって「オーバーレイの音色」はその track の init meas の音色そのもの。
//! ただし**確定したときだけ**で、音色一覧をカーソルで流している間（preview）は
//! 変えてはいけない。preview のたびに init セルを書くと再レンダリングが暴発する。

use crossterm::event::KeyCode;

use cmrt_mml_overlay::MmlOverlayAction;
use cmrt_tui_core::patch_load::PatchLoadState;

use super::super::super::{DawApp, DawMode};
use super::{ctrl, key, plain};
use crate::input::tests::build_test_app;

const PAD_INIT_CELL: &str = r#"{"Surge XT patch": "Pads/Snapshot Pad.fxp"}"#;
/// builtin の分類にどれも当たらない音色。ユーザープリセットの効果だけを見るため。
const UNCLASSIFIED_PATCH: &str = "Misc/Zzq Item.fxp";

fn catalog_pairs() -> Vec<(String, String)> {
    [
        "Bass/Snapshot Bass.fxp",
        "Pads/Snapshot Pad.fxp",
        UNCLASSIFIED_PATCH,
    ]
    .into_iter()
    .map(|display| (display.to_string(), display.to_lowercase()))
    .collect()
}

/// 音色一覧の絞り込み欄へ打ち込んで、確定する音色を 1 つに絞る。
/// 一覧はカーソルが「いまの音色」の上で開くので、絞らないと何が確定するかが
/// 一覧の並び順に依存してしまう。
fn confirm_patch_by_query(app: &mut DawApp, query: &str) {
    app.handle_mml_overlay_key_event(ctrl('t'));
    assert!(app.mml_overlay.is_patch_select_open());
    for ch in query.chars() {
        app.handle_mml_overlay_key_event(plain(ch));
    }
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));
}

/// 注入 snapshot 付きで、track1 の meas1 を開いた状態にする。
/// init セルには `Pads/...` を入れてあるので、`Bass/...` を確定すれば必ず値が変わる。
fn opened_with_the_pad_patch() -> (DawApp, std::sync::mpsc::Receiver<crate::CacheJob>) {
    let (mut app, cache_rx) = build_test_app();
    *app.patch_load.lock().unwrap() = PatchLoadState::ready(catalog_pairs());
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] = PAD_INIT_CELL.to_string();
    app.editor.data[2][1] = "cde".to_string();
    assert!(app.try_open_mml_overlay(ctrl('p')));
    (app, cache_rx)
}

#[test]
fn confirming_a_patch_writes_it_into_the_init_cell() {
    let (mut app, _cache_rx) = opened_with_the_pad_patch();

    confirm_patch_by_query(&mut app, "snapshot bass");

    assert_eq!(app.mml_overlay.patch(), Some("Bass/Snapshot Bass.fxp"));
    assert_eq!(
        app.editor.data[2][0],
        r#"{"Surge XT patch": "Bass/Snapshot Bass.fxp"}"#
    );
}

#[test]
fn previewing_a_patch_does_not_touch_the_init_cell() {
    let (mut app, _cache_rx) = opened_with_the_pad_patch();

    app.handle_mml_overlay_key_event(ctrl('t'));
    // 一覧をカーソルで流すと鳴らす先だけが変わる（`SetPatch`）。確定はしていない。
    // preview がそもそも起きていなければこのテストは何も守らないので、先に確かめる。
    let action = app
        .mml_overlay
        .handle_key(key(KeyCode::Up), std::time::Instant::now());
    assert!(
        matches!(action, MmlOverlayAction::SetPatch { .. }),
        "カーソル移動は試聴を求めるはず: {action:?}"
    );
    app.handle_mml_overlay_key_event(key(KeyCode::Down));

    assert_eq!(
        app.editor.data[2][0], PAD_INIT_CELL,
        "preview では init セルを書き換えないこと"
    );
    assert_eq!(app.mml_overlay.patch(), Some("Pads/Snapshot Pad.fxp"));
}

#[test]
fn confirming_a_patch_keeps_the_patch_filter_query() {
    let (mut app, _cache_rx) = opened_with_the_pad_patch();
    app.editor.data[2][0] =
        r#"{"Surge XT patch": "Pads/Snapshot Pad.fxp", "Surge XT patch filter": "snapshot"}"#
            .to_string();

    confirm_patch_by_query(&mut app, "snapshot bass");

    assert_eq!(
        app.editor.data[2][0],
        r#"{"Surge XT patch": "Bass/Snapshot Bass.fxp", "Surge XT patch filter": "snapshot"}"#,
        "音色名だけを差し替え、付随メタデータは壊さないこと"
    );
}

#[test]
fn the_tempo_track_init_cell_is_never_touched() {
    let (mut app, _cache_rx) = build_test_app();
    *app.patch_load.lock().unwrap() = PatchLoadState::ready(catalog_pairs());
    app.editor.cursor_track = 0;
    app.editor.cursor_measure = 1;
    app.editor.data[0][0] = r#"{"beat": "4/4"}t120"#.to_string();
    assert!(app.try_open_mml_overlay(ctrl('p')));

    confirm_patch_by_query(&mut app, "snapshot bass");

    assert_eq!(
        app.editor.data[0][0], r#"{"beat": "4/4"}t120"#,
        "Tempo 行の init セルは拍子とテンポ。音色を書き込んではいけない"
    );
}

/// 確定した音色は、閉じたあとも init セルに残る（＝ DAW で実際に鳴る音色になる）。
#[test]
fn the_confirmed_patch_survives_closing_the_overlay() {
    let (mut app, _cache_rx) = opened_with_the_pad_patch();
    confirm_patch_by_query(&mut app, "snapshot bass");

    app.handle_mml_overlay_key_event(key(KeyCode::Esc));

    assert_eq!(app.mode, DawMode::Normal);
    assert_eq!(
        app.editor.data[2][0],
        r#"{"Surge XT patch": "Bass/Snapshot Bass.fxp"}"#
    );
}

/// 音色フィルタのプリセット追加は保存され、role の索引も作り直される。
/// 作り直さないと init 列の `role:音色名` 表示（Stage 4）が追従しない。
#[test]
fn adding_a_patch_filter_preset_rebuilds_the_role_index() {
    let (mut app, _cache_rx) = opened_with_the_pad_patch();
    // プリセットの保存先はテスト用の temp ディレクトリだが、プロセス内で共有される。
    // 他のテストへ漏らさないよう、最後に書き戻す。
    let saved = cmrt_history::load_mml_patch_filter_presets();
    let display = UNCLASSIFIED_PATCH;
    let presets = vec![("bass".to_string(), "zzq".to_string())];
    assert_ne!(
        app.catalog_snapshot()
            .and_then(|snapshot| snapshot.patch_roles().role_of(display)),
        Some(cmrt_patches::PatchRole::Bass)
    );

    app.apply_mml_overlay_action(MmlOverlayAction::SavePatchFilterPresets {
        presets: presets.clone(),
        preview: None,
    });

    assert_eq!(cmrt_history::load_mml_patch_filter_presets(), presets);
    assert_eq!(
        app.catalog_snapshot()
            .and_then(|snapshot| snapshot.patch_roles().role_of(display)),
        Some(cmrt_patches::PatchRole::Bass),
        "追加したプリセットが注入 snapshot の role 索引へ反映されるはず"
    );

    cmrt_history::save_mml_patch_filter_presets(&saved).expect("プリセットを書き戻せるはず");
}

/// 音色を変えたら、その track の書かれている meas は鳴らし直しの対象になる。
/// `r`（ランダム音色）と同じ 1 実装を通ることを、ログと予約されたジョブで確かめる。
#[test]
fn confirming_a_patch_rerenders_the_measures_of_the_track() {
    let (mut app, cache_rx) = opened_with_the_pad_patch();
    app.editor.data[2][2] = "gab".to_string();

    confirm_patch_by_query(&mut app, "snapshot bass");

    let logs = super::log_lines(&app);
    assert!(
        logs.iter()
            .any(|line| line == "cache: rerender start track1 meas 1〜2 (mml overlay patch update)"),
        "logs: {logs:?}"
    );
    assert!(
        logs.iter()
            .any(|line| line.starts_with("play: hot reload mml overlay patch track1 ")),
        "logs: {logs:?}"
    );
    assert!(
        cache_rx.try_recv().is_ok(),
        "鳴らし直しのジョブが予約されるはず"
    );
}
