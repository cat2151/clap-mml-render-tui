use super::*;

const INIT_WITH_ROOM_AND_CLEAN: &str = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Reverb 2/Room"},{"Test Amp preset":"Clean"}]}"#;

#[test]
fn r_enter_replaces_the_cursor_stage_and_keeps_the_other_stages() {
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[2][0] = INIT_WITH_ROOM_AND_CLEAN.to_string();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('r')]);
    assert!(matches!(app.mode, DawMode::EffectChainAdd));
    assert_eq!(app.overlays.effect_chain.add.replace_target, Some(0));

    // list 先頭は Delay/Echo。
    press(&mut app, &[KeyCode::Enter]);

    assert!(matches!(app.mode, DawMode::EffectChain));
    assert_eq!(
        app.overlays.effect_chain.chain,
        vec![
            json!({"Test FX preset": "Delay/Echo"}),
            json!({"Test Amp preset": "Clean"}),
        ]
    );
    assert_eq!(app.overlays.effect_chain.cursor, 0);
}

#[test]
fn r_on_an_empty_chain_does_nothing() {
    let (mut app, _cache_rx) = app_with_catalog();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('r')]);

    assert!(matches!(app.mode, DawMode::EffectChain));
}

#[test]
fn a_after_r_appends_again() {
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[2][0] = INIT_WITH_ROOM_AND_CLEAN.to_string();

    press(
        &mut app,
        &[
            KeyCode::Char('x'),
            KeyCode::Char('r'),
            KeyCode::Esc,
            KeyCode::Char('a'),
            KeyCode::Enter,
        ],
    );

    assert_eq!(app.overlays.effect_chain.chain.len(), 3);
    assert_eq!(
        app.overlays.effect_chain.chain[2],
        json!({"Test FX preset": "Delay/Echo"})
    );
}

#[test]
fn replace_overlay_previews_the_chain_with_the_candidate_in_place_of_the_cursor_stage() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    app.editor.data[2][0] = INIT_WITH_ROOM_AND_CLEAN.to_string();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('r')]);

    assert_eq!(
        live_chain(&wait_for_live_line(&sink, 1)),
        json!([
            {"Test FX preset": "Delay/Echo"},
            {"Test Amp preset": "Clean"}
        ])
    );
}

#[test]
fn b_in_the_replace_overlay_previews_with_only_the_candidate_stage_bypassed() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    app.editor.data[2][0] = INIT_WITH_ROOM_AND_CLEAN.to_string();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('r')]);
    wait_for_live_line(&sink, 1);
    press(&mut app, &[KeyCode::Char('b')]);

    assert!(matches!(app.mode, DawMode::EffectChainAdd));
    assert_eq!(
        live_chain(&wait_for_live_line(&sink, 2)),
        json!([
            {"Test FX preset": "Delay/Echo", "bypass": true},
            {"Test Amp preset": "Clean"}
        ])
    );
    // b は preview だけで、編集中の chain は変えない。
    assert_eq!(
        app.overlays.effect_chain.chain[0],
        json!({"Test FX preset": "Reverb 2/Room"})
    );
}

#[test]
fn b_in_the_add_overlay_previews_with_only_the_appended_candidate_bypassed() {
    let (mut app, _cache_rx) = app_with_catalog();
    let sink = attach_live(&mut app, RecordingSink::default());
    app.editor.data[2][0] = INIT_WITH_ROOM_AND_CLEAN.to_string();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    wait_for_live_line(&sink, 1);
    press(&mut app, &[KeyCode::Char('b')]);

    assert_eq!(
        live_chain(&wait_for_live_line(&sink, 2)),
        json!([
            {"Test FX preset": "Reverb 2/Room"},
            {"Test Amp preset": "Clean"},
            {"Test FX preset": "Delay/Echo", "bypass": true}
        ])
    );
    assert_eq!(app.overlays.effect_chain.chain.len(), 2);
}
