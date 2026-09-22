use super::*;

fn preview_json_and_phrase(app: &DawApp, track: usize) -> (serde_json::Value, String) {
    let track_mmls = app.playback.measure_track_mmls.lock().unwrap()[0].clone();
    DawApp::extract_patch_json_and_phrase(&track_mmls[track]).unwrap()
}

#[test]
fn space_previews_the_bypass_reflecting_chain_on_the_overlay_track_only() {
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Reverb 1/Hall"}]}"#.to_string();

    press(
        &mut app,
        &[KeyCode::Char('x'), KeyCode::Char('b'), KeyCode::Char(' ')],
    );

    assert!(matches!(app.mode, DawMode::EffectChain));
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    let (json, phrase) = preview_json_and_phrase(&app, 2);
    assert_eq!(
        json,
        json!({
            "Surge XT patch": "Pads/Pad 1.fxp",
            "effects after instrument": [{"Test FX preset": "Reverb 1/Hall", "bypass": true}],
        })
    );
    assert_eq!(phrase, "cdef");
    let track_mmls = app.playback.measure_track_mmls.lock().unwrap()[0].clone();
    for (track, mml) in track_mmls.iter().enumerate() {
        if track != 2 {
            assert!(mml.is_empty(), "track {track} は無音のはず: {mml:?}");
        }
    }
    // Enter していないので init セルはまだ書き換わっていない（bypass 無しのまま）。
    assert_eq!(
        app.editor.data[2][0],
        r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Reverb 1/Hall"}]}"#
    );
}

#[test]
fn space_in_the_add_overlay_previews_chain_plus_cursor_preset_without_committing() {
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Reverb 1/Hall"}]}"#.to_string();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    assert_eq!(app.overlays.effect_chain.add.list_cursor, 0);
    press(&mut app, &[KeyCode::Char(' ')]);

    assert!(matches!(app.mode, DawMode::EffectChainAdd));
    // 追加 overlay の Space は state.chain を変えない。
    assert_eq!(app.overlays.effect_chain.chain.len(), 1);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    let (json, _) = preview_json_and_phrase(&app, 2);
    let chain = json
        .get("effects after instrument")
        .and_then(serde_json::Value::as_array)
        .expect("chain array");
    assert_eq!(chain.len(), 2, "chain: {chain:?}");
}

#[test]
fn space_does_nothing_while_playing() {
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Reverb 1/Hall"}]}"#.to_string();
    app.handle_normal(KeyCode::Char('x'));
    *app.playback.play_state.lock().unwrap() = DawPlayState::Playing;
    let before = app.playback.measure_track_mmls.lock().unwrap().clone();

    app.handle_effect_chain(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

    assert_eq!(*app.playback.measure_track_mmls.lock().unwrap(), before);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Playing
    ));
}

#[test]
fn space_previews_the_chord_generated_phrase_when_the_cell_is_empty() {
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[crate::CHORD_TRACK][1] = "IIm7".to_string();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","generate from chord track":"close","effects after instrument":[{"Test FX preset":"Reverb 1/Hall"}]}"#.to_string();
    app.editor.data[2][1] = String::new();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char(' ')]);

    let expected =
        crate::mml::build_cell_mml_from_data(&app.editor.data, app.editor.measures, 2, 1);
    let (_, expected_phrase) = DawApp::extract_patch_json_and_phrase(&expected).unwrap();
    let (_, phrase) = preview_json_and_phrase(&app, 2);
    assert_ne!(
        phrase, "c",
        "chord 行から生成される小節を空セル扱いして fallback にしてはいけない"
    );
    assert_eq!(phrase, expected_phrase);
}

fn preview_chain(app: &DawApp) -> Vec<serde_json::Value> {
    let (json, _) = preview_json_and_phrase(app, 2);
    json.get("effects after instrument")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// 追加 overlay は開いた時点と、候補が変わるカーソル移動のたびに自動で preview する。
/// 候補は `Delay/Echo`、`Reverb 2/Room`、`Clean` の順（`app_with_catalog` の catalog）。
#[test]
fn add_overlay_previews_on_open_and_when_the_cursor_changes_the_candidate() {
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Reverb 1/Hall"}]}"#.to_string();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        preview_chain(&app),
        vec![
            json!({"Test FX preset": "Reverb 1/Hall"}),
            json!({"Test FX preset": "Delay/Echo"})
        ]
    );

    press(&mut app, &[KeyCode::Char('j')]);
    assert_eq!(
        preview_chain(&app),
        vec![
            json!({"Test FX preset": "Reverb 1/Hall"}),
            json!({"Test FX preset": "Reverb 2/Room"})
        ]
    );

    // 隣の候補は先読みされて overlay preview cache に入る。
    assert_ne!(app.render.preview_cache().entry_count(), 0);

    // state.chain は Enter まで変わらない。
    assert_eq!(app.overlays.effect_chain.chain.len(), 1);
}

/// `h`/`l`（pane 切替）と、端で止まる `k` は候補を変えないので鳴らし直さない。
#[test]
fn add_overlay_does_not_repreview_when_the_candidate_stays() {
    let (mut app, _cache_rx) = app_with_catalog();
    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    app.stop_play();
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));

    press(
        &mut app,
        &[KeyCode::Char('k'), KeyCode::Char('h'), KeyCode::Char('l')],
    );

    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
}

/// kind pane の移動で list が絞り直されて候補が変われば、その候補で preview する。
#[test]
fn add_overlay_previews_after_a_kind_change_rebuilds_the_list() {
    let (mut app, _cache_rx) = app_with_catalog();
    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    app.stop_play();

    // kinds は `all`, `Amp Simulator`, `Delay`, `Reverb`（sort 済み）。`h` で kind pane へ
    // focus し、`j` で `Amp Simulator` へ。
    press(&mut app, &[KeyCode::Char('h'), KeyCode::Char('j')]);

    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        preview_chain(&app),
        vec![json!({"Test Amp preset": "Clean"})]
    );
}

fn stop_and_assert_idle(app: &mut DawApp) {
    app.stop_play();
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
}

fn assert_previewing(app: &DawApp) {
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
}

/// chain が変わる操作（`b`・`dd`・`Alt+↓`）の直後は、その chain で自動 preview する。
#[test]
fn chain_list_previews_after_bypass_delete_and_reorder() {
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Reverb 1/Hall"},{"Test FX preset":"Delay/Echo"}]}"#.to_string();
    press(&mut app, &[KeyCode::Char('x')]);

    press(&mut app, &[KeyCode::Char('b')]);
    assert_previewing(&app);
    assert_eq!(
        preview_chain(&app),
        vec![
            json!({"Test FX preset": "Reverb 1/Hall", "bypass": true}),
            json!({"Test FX preset": "Delay/Echo"}),
        ]
    );

    stop_and_assert_idle(&mut app);
    app.handle_effect_chain(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT));
    assert_previewing(&app);
    assert_eq!(
        preview_chain(&app),
        vec![
            json!({"Test FX preset": "Delay/Echo"}),
            json!({"Test FX preset": "Reverb 1/Hall", "bypass": true}),
        ]
    );

    stop_and_assert_idle(&mut app);
    press(&mut app, &[KeyCode::Char('d'), KeyCode::Char('d')]);
    assert_previewing(&app);
    assert_eq!(
        preview_chain(&app),
        vec![json!({"Test FX preset": "Delay/Echo"})]
    );
}

/// chain を変えない操作（`j`/`k`、端で止まる `Alt+↑`、空 chain の `b`/`dd`）は鳴らさない。
#[test]
fn chain_list_does_not_preview_when_the_chain_stays() {
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Reverb 1/Hall"}]}"#.to_string();
    press(
        &mut app,
        &[KeyCode::Char('x'), KeyCode::Char('j'), KeyCode::Char('k')],
    );
    app.handle_effect_chain(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));

    let (mut app, _cache_rx) = app_with_catalog();
    press(
        &mut app,
        &[
            KeyCode::Char('x'),
            KeyCode::Char('b'),
            KeyCode::Char('d'),
            KeyCode::Char('d'),
        ],
    );
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
}
