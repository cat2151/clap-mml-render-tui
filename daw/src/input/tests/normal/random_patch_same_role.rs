//! `r`（ランダム音色）が、いま鳴っている音色と同じ用途へ寄ることの検証。
//!
//! kick の track で pad が出てくると曲の作りが壊れるため、role を揃える。
//! drum は role の中の部位（kick / snare / hat / perc）まで揃える。

use super::*;

use cmrt_tui_core::patch_load::PatchLoadState;

/// 走査経路へ落ちないよう、実在しない patch dir を指す Config へ差し替える。
/// 候補は注入した snapshot だけになるので、抽選結果が分類だけで決まる。
fn point_config_at_missing_patch_dir(app: &mut DawApp) {
    let missing = std::env::temp_dir().join("cmrt_test_daw_same_role_missing_patch_dir");
    std::fs::remove_dir_all(&missing).ok();
    app.cfg = Arc::new(Config {
        patches_dirs: Some(vec![missing.to_string_lossy().into_owned()]),
        ..(*app.cfg).clone()
    });
}

fn pair(display: &str) -> (String, String) {
    (display.to_string(), display.to_lowercase())
}

fn catalog() -> Vec<(String, String)> {
    vec![
        pair("Drums/Kick Clean.fxp"),
        pair("Drums/Kick Punch.fxp"),
        pair("Drums/Snare Tight.fxp"),
        pair("Drums/Closed Hat.fxp"),
        pair("Bass/Deep Bass.fxp"),
        pair("Bass/Sub Bass.fxp"),
        pair("Pads/Warm Pad.fxp"),
        pair("Leads/Bright Lead.fxp"),
    ]
}

fn selected_patch(app: &DawApp) -> String {
    let init_json: serde_json::Value = serde_json::from_str(&app.editor.data[2][0]).unwrap();
    init_json["Surge XT patch"]
        .as_str()
        .expect("selected patch should be stored as string")
        .to_string()
}

/// snapshot を注入した app で、track 2 の init セルを `patch` にしてから `r` を n 回押す。
fn press_r_from(tmp_name: &str, patch: &str, times: usize) -> Vec<String> {
    let tmp = std::env::temp_dir().join(tmp_name);
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();

    let selected = {
        let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);

        let (mut app, _cache_rx) = build_test_app();
        point_config_at_missing_patch_dir(&mut app);
        *app.patch_load.lock().unwrap() = PatchLoadState::ready(catalog());
        app.editor.cursor_track = 2;
        app.editor.cursor_measure = 0;
        app.editor.data[2][0] = format!(r#"{{"Surge XT patch":"{patch}"}}"#);

        (0..times)
            .map(|_| {
                app.handle_normal(crossterm::event::KeyCode::Char('r'));
                selected_patch(&app)
            })
            .collect::<Vec<_>>()
    };

    std::fs::remove_dir_all(&tmp).ok();
    selected
}

#[test]
fn handle_normal_r_keeps_the_drum_part_of_the_current_patch() {
    let selected = press_r_from(
        "cmrt_test_daw_random_patch_same_role_kick",
        "Drums/Kick Clean.fxp",
        8,
    );

    for patch in &selected {
        assert!(
            matches!(
                patch.as_str(),
                "Drums/Kick Clean.fxp" | "Drums/Kick Punch.fxp"
            ),
            "kick の track では kick だけが選ばれるはず: {patch}"
        );
    }
}

#[test]
fn handle_normal_r_keeps_the_role_of_the_current_patch() {
    let selected = press_r_from(
        "cmrt_test_daw_random_patch_same_role_bass",
        "Bass/Deep Bass.fxp",
        8,
    );

    for patch in &selected {
        assert!(
            matches!(patch.as_str(), "Bass/Deep Bass.fxp" | "Bass/Sub Bass.fxp"),
            "bass の track では bass だけが選ばれるはず: {patch}"
        );
    }
}

/// role を絞っても、その中では従来どおりの一巡抽選（同じ音色を続けて引かない）。
#[test]
fn handle_normal_r_still_cycles_within_the_same_role() {
    let selected = press_r_from(
        "cmrt_test_daw_random_patch_same_role_cycle",
        "Drums/Kick Clean.fxp",
        2,
    );

    assert_ne!(
        selected[0], selected[1],
        "同じ role の中でも 2 連続で同じ音色は引かないはず: {selected:?}"
    );
}

/// catalog が知らない音色の track では、用途を絞れないので従来どおり全体から抽選する。
#[test]
fn handle_normal_r_falls_back_to_the_whole_catalog_for_an_unknown_patch() {
    let selected = press_r_from(
        "cmrt_test_daw_random_patch_same_role_unknown",
        "Missing/Not In Catalog.fxp",
        1,
    );

    assert!(
        catalog().iter().any(|(display, _)| *display == selected[0]),
        "分類できない音色でも抽選自体は成立するはず: {selected:?}"
    );
}

/// 実機の config.toml を渡したときだけ走る、実カタログでの確認。
///
/// 開発機のインストール状況に依存するので、環境変数が無ければ skip する
/// （個人のパスをコードへ書かないため、パスは環境変数で渡す）。
///
/// ```text
/// CMRT_TEST_DAW_REAL_CONFIG=%LOCALAPPDATA%\clap-mml-render-tui\config.toml \
///   cargo test -p cmrt-daw real_catalog_kick -- --nocapture
/// ```
#[test]
fn real_catalog_kick_track_keeps_drawing_kicks() {
    let Some(config_path) = std::env::var_os("CMRT_TEST_DAW_REAL_CONFIG") else {
        eprintln!("skip: CMRT_TEST_DAW_REAL_CONFIG が未設定");
        return;
    };
    let cfg = Config::load_from_path(std::path::Path::new(&config_path))
        .expect("CMRT_TEST_DAW_REAL_CONFIG の config.toml を読めること");
    let pairs = cmrt_tui_core::patches::collect_patch_pairs(&cfg).expect("実カタログの走査");
    let snapshot = cmrt_tui_core::patch_load::PatchCatalogSnapshot::from_pairs(pairs.clone());
    let kicks = snapshot
        .patch_roles()
        .drum_candidates(cmrt_patches::DrumPatchRole::Kick)
        .to_vec();
    assert!(!kicks.is_empty(), "実カタログに kick が 1 件も無い");

    let tmp = std::env::temp_dir().join("cmrt_test_daw_real_catalog_kick");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();
    {
        let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);

        let (mut app, _cache_rx) = build_test_app();
        app.cfg = Arc::new(cfg);
        *app.patch_load.lock().unwrap() = PatchLoadState::ready(pairs);
        app.editor.cursor_track = 2;
        app.editor.cursor_measure = 0;
        app.editor.data[2][0] = format!(r#"{{"Surge XT patch":"{}"}}"#, kicks[0]);

        for _ in 0..20 {
            app.handle_normal(crossterm::event::KeyCode::Char('r'));
            let picked = selected_patch(&app);
            assert!(
                kicks.contains(&picked),
                "kick の track で kick 以外が選ばれた: {picked}"
            );
        }
        eprintln!("real catalog: kick candidates={}", kicks.len());
    }
    std::fs::remove_dir_all(&tmp).ok();
}
