use super::*;

const INIT_WITH_HALL: &str = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Reverb 1/Hall"}]}"#;

#[test]
fn space_previews_the_bypass_reflecting_chain_on_the_overlay_track_only() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    app.editor.data[2][0] = INIT_WITH_HALL.to_string();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('b')]);
    wait_for_live_line(&sink, 1);
    press(&mut app, &[KeyCode::Char(' ')]);

    assert!(matches!(app.mode, DawMode::EffectChain));
    // cache に無いので LIVE で鳴らす。offline の preview は始めない。
    assert_idle(&app);
    let live = wait_for_live_line(&sink, 2);
    assert_eq!(live.patch(), Some("Pads/Pad 1.fxp"));
    assert_eq!(
        live_chain(&live),
        json!([{"Test FX preset": "Reverb 1/Hall", "bypass": true}])
    );

    let chain = app.overlays.effect_chain.chain.clone();
    let (_, track_mmls) = app.effect_chain_preview_track_mmls(&chain);
    let (_, phrase) = DawApp::extract_patch_json_and_phrase(&track_mmls[2]).unwrap();
    assert_eq!(phrase, "cdef");
    for (track, mml) in track_mmls.iter().enumerate() {
        if track != 2 {
            assert!(mml.is_empty(), "track {track} は無音のはず: {mml:?}");
        }
    }
    // Enter していないので init セルはまだ書き換わっていない（bypass 無しのまま）。
    assert_eq!(app.editor.data[2][0], INIT_WITH_HALL);
}

#[test]
fn space_in_the_add_overlay_previews_chain_plus_cursor_preset_without_committing() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    app.editor.data[2][0] = INIT_WITH_HALL.to_string();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    assert_eq!(app.overlays.effect_chain.add.list_cursor, 0);
    wait_for_live_line(&sink, 1);
    press(&mut app, &[KeyCode::Char(' ')]);

    assert!(matches!(app.mode, DawMode::EffectChainAdd));
    // 追加 overlay の Space は state.chain を変えない。
    assert_eq!(app.overlays.effect_chain.chain.len(), 1);
    assert_eq!(
        live_chain(&wait_for_live_line(&sink, 2)),
        json!([
            {"Test FX preset": "Reverb 1/Hall"},
            {"Test FX preset": "Delay/Echo"}
        ])
    );
}

#[test]
fn opening_overlay_stops_playing_so_space_previews() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    app.editor.data[2][0] = INIT_WITH_HALL.to_string();
    *app.playback.play_state.lock().unwrap() = DawPlayState::Playing;

    app.handle_normal(KeyCode::Char('x'));

    assert!(matches!(app.mode, DawMode::EffectChain));
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    app.handle_effect_chain(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert_eq!(
        live_chain(&wait_for_live_line(&sink, 1)),
        json!([{"Test FX preset": "Reverb 1/Hall"}])
    );
}

#[test]
fn space_does_nothing_while_playing() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    app.editor.data[2][0] = INIT_WITH_HALL.to_string();
    app.handle_normal(KeyCode::Char('x'));
    *app.playback.play_state.lock().unwrap() = DawPlayState::Playing;
    let before = app.playback.measure_track_mmls.lock().unwrap().clone();

    app.handle_effect_chain(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

    assert_eq!(*app.playback.measure_track_mmls.lock().unwrap(), before);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Playing
    ));
    assert_no_more_live_lines(&sink, 0);
}

#[test]
fn space_previews_the_chord_generated_phrase_when_the_cell_is_empty() {
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[crate::CHORD_TRACK][1] = "IIm7".to_string();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","generate from chord track":"close","effects after instrument":[{"Test FX preset":"Reverb 1/Hall"}]}"#.to_string();
    app.editor.data[2][1] = String::new();

    press(&mut app, &[KeyCode::Char('x')]);
    let chain = app.overlays.effect_chain.chain.clone();
    let (_, track_mmls) = app.effect_chain_preview_track_mmls(&chain);

    let expected =
        crate::mml::build_cell_mml_from_data(&app.editor.data, app.editor.measures, 2, 1);
    let (_, expected_phrase) = DawApp::extract_patch_json_and_phrase(&expected).unwrap();
    let (_, phrase) = DawApp::extract_patch_json_and_phrase(&track_mmls[2]).unwrap();
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
///
/// 開いた時点の候補は cache に無いので LIVE で鳴り、現在の候補は render しない。
/// 隣の候補は先読み render されて cache に入り、`j` で移るとそこから鳴る（LIVE へは送らない）。
#[test]
fn add_overlay_previews_on_open_and_when_the_cursor_changes_the_candidate() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    app.editor.data[2][0] = INIT_WITH_HALL.to_string();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    assert_idle(&app);
    assert_eq!(
        live_chain(&wait_for_live_line(&sink, 1)),
        json!([
            {"Test FX preset": "Reverb 1/Hall"},
            {"Test FX preset": "Delay/Echo"}
        ])
    );
    assert!(
        app.render.preview_cache().entry_count() > 0,
        "隣の候補は従来どおり先読みする"
    );
    let current_chain = app.effect_chain_add_candidate_chain(0).unwrap();
    let (measure_index, track_mmls) = app.effect_chain_preview_track_mmls(&current_chain);
    let track_gains = app.effect_chain_preview_track_gains();
    let current = crate::preview::service::OfflinePreviewRequest::current(
        measure_index,
        app.measure_duration_samples(),
        crate::preview::active_preview_tracks(&track_mmls, &track_gains),
        track_mmls,
        track_gains,
    );
    assert!(
        app.render.preview_service().cached(&current).is_none(),
        "LIVE で鳴らした現在の候補は render しない"
    );

    press(&mut app, &[KeyCode::Char('j')]);
    assert_previewing(&app);
    assert_eq!(
        preview_chain(&app),
        vec![
            json!({"Test FX preset": "Reverb 1/Hall"}),
            json!({"Test FX preset": "Reverb 2/Room"})
        ]
    );
    assert!(app.overlays.effect_chain.live_preview_command.is_none());
    assert_no_more_live_lines(&sink, 1);

    // state.chain は Enter まで変わらない。
    assert_eq!(app.overlays.effect_chain.chain.len(), 1);
}

/// `h`/`l`（pane 切替）と、端で止まる `k` は候補を変えないので鳴らし直さない。
#[test]
fn add_overlay_does_not_repreview_when_the_candidate_stays() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    wait_for_live_line(&sink, 1);

    press(
        &mut app,
        &[KeyCode::Char('k'), KeyCode::Char('h'), KeyCode::Char('l')],
    );

    assert_idle(&app);
    assert_no_more_live_lines(&sink, 1);
}

/// kind pane の移動で list が絞り直されて候補が変われば、その候補で preview する。
/// `Clean` は開いた時点で隣の候補として先読みされているので、cache から鳴る。
#[test]
fn add_overlay_previews_after_a_kind_change_rebuilds_the_list() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    wait_for_live_line(&sink, 1);

    // kinds は `all`, `Amp Simulator`, `Delay`, `Reverb`（sort 済み）。`h` で kind pane へ
    // focus し、`j` で `Amp Simulator` へ。
    press(&mut app, &[KeyCode::Char('h'), KeyCode::Char('j')]);

    assert_previewing(&app);
    assert_eq!(
        preview_chain(&app),
        vec![json!({"Test Amp preset": "Clean"})]
    );
    assert_no_more_live_lines(&sink, 1);
}

/// chain が変わる操作（`b`・`dd`・`Alt+↓`）の直後は、その chain で自動 preview する。
#[test]
fn chain_list_previews_after_bypass_delete_and_reorder() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Reverb 1/Hall"},{"Test FX preset":"Delay/Echo"}]}"#.to_string();
    press(&mut app, &[KeyCode::Char('x')]);

    press(&mut app, &[KeyCode::Char('b')]);
    assert_eq!(
        live_chain(&wait_for_live_line(&sink, 1)),
        json!([
            {"Test FX preset": "Reverb 1/Hall", "bypass": true},
            {"Test FX preset": "Delay/Echo"},
        ])
    );

    app.handle_effect_chain(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT));
    assert_eq!(
        live_chain(&wait_for_live_line(&sink, 2)),
        json!([
            {"Test FX preset": "Delay/Echo"},
            {"Test FX preset": "Reverb 1/Hall", "bypass": true},
        ])
    );

    press(&mut app, &[KeyCode::Char('d'), KeyCode::Char('d')]);
    assert_eq!(
        live_chain(&wait_for_live_line(&sink, 3)),
        json!([{"Test FX preset": "Delay/Echo"}])
    );
}

/// chain を変えない操作（`j`/`k`、端で止まる `Alt+↑`、空 chain の `b`/`dd`）は鳴らさない。
#[test]
fn chain_list_does_not_preview_when_the_chain_stays() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    app.editor.data[2][0] = INIT_WITH_HALL.to_string();
    press(
        &mut app,
        &[KeyCode::Char('x'), KeyCode::Char('j'), KeyCode::Char('k')],
    );
    app.handle_effect_chain(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
    assert_idle(&app);
    assert_no_more_live_lines(&sink, 0);

    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    press(
        &mut app,
        &[
            KeyCode::Char('x'),
            KeyCode::Char('b'),
            KeyCode::Char('d'),
            KeyCode::Char('d'),
        ],
    );
    assert_idle(&app);
    assert_no_more_live_lines(&sink, 0);
}

/// 準備（音色・chain）の失敗は log と overlay に 1 行出し、その試聴は鳴らさない。次の試聴で消す。
#[test]
fn a_failed_live_preparation_is_shown_until_the_next_preview() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(
        &mut app,
        RecordingSink::failing_prepare("chain を作れません"),
    );
    app.editor.data[2][0] = INIT_WITH_HALL.to_string();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char(' ')]);
    wait_until("準備の失敗", || {
        app.pump_effect_chain_live_preview();
        app.overlays.effect_chain.preview_error.is_some()
    });

    let error = app.overlays.effect_chain.preview_error.clone().unwrap();
    assert!(error.contains("chain を作れません"), "error: {error}");
    assert_eq!(sink.timelines(), 0, "準備に失敗した試聴は鳴らさない");
    assert_idle(&app);
    assert!(app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line.contains("chain を作れません")));

    press(&mut app, &[KeyCode::Char(' ')]);
    assert_eq!(app.overlays.effect_chain.preview_error, None);
}

/// 通常の preview と DAW の演奏は、LIVE の試聴を止めてから始まる。
#[test]
fn a_normal_preview_and_play_stop_the_live_preview() {
    let starts: [fn(&mut DawApp); 2] = [|app| app.start_preview(0), |app| app.start_play()];
    for start in starts {
        let (mut app, _cache_rx) = app_with_catalog();
        let sink = attach_live(&mut app, RecordingSink::default());
        app.editor.data[2][0] = INIT_WITH_HALL.to_string();
        press(&mut app, &[KeyCode::Char('x'), KeyCode::Char(' ')]);
        wait_for_live_line(&sink, 1);
        assert_eq!(sink.stops(), 0);

        start(&mut app);

        wait_until("LIVE の試聴の停止", || sink.stops() >= 1);
        assert_eq!(
            sink.fade_outs(),
            0,
            "fadeout は EFFECT CHAIN の試聴の移動だけ"
        );
    }
}

fn fade_out_of_the_first_line() -> SinkOperation {
    SinkOperation::FadeOut {
        instance_ids: vec![0],
        fade_ms: 50,
    }
}

/// LIVE の候補から LIVE の候補へ移ると、前の候補を 50 ms で fadeout してから次の候補を準備する。
/// 全NoteOff（fadeout を段差で切る）は送らない。
#[test]
fn moving_between_live_candidates_fades_out_the_previous_one_before_preparing() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    app.editor.data[2][0] = INIT_WITH_HALL.to_string();
    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('b')]);
    wait_for_live_line(&sink, 1);

    press(&mut app, &[KeyCode::Char('b')]);
    wait_for_live_line(&sink, 2);

    assert_eq!(
        sink.operations(),
        vec![
            SinkOperation::Prepare,
            SinkOperation::Timeline,
            fade_out_of_the_first_line(),
            SinkOperation::Prepare,
            SinkOperation::Timeline,
        ]
    );
}

/// LIVE の候補から cache の候補へ移っても、LIVE の前の候補を fadeout する（止めない）。
#[test]
fn moving_from_a_live_candidate_to_a_cached_one_fades_out_the_live_one() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    app.editor.data[2][0] = INIT_WITH_HALL.to_string();
    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    wait_for_live_line(&sink, 1);

    press(&mut app, &[KeyCode::Char('j')]);
    assert_previewing(&app);
    assert_no_more_live_lines(&sink, 1);

    assert_eq!(
        sink.operations(),
        vec![
            SinkOperation::Prepare,
            SinkOperation::Timeline,
            fade_out_of_the_first_line(),
        ]
    );
}
