use super::*;
use cmrt_core::{AudioEffectCatalog, AudioEffectPluginInfo, AudioEffectPreset};
use cmrt_offline_render::{EffectPlugins, PluginEntries};
use serde_json::json;

const INIT_WITH_PATCH: &str = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#;

/// マシンに依存しない、手で並べた catalog（plugin 1 つ、preset 2 つ）。
fn test_catalog() -> AudioEffectCatalog {
    let plugin = AudioEffectPluginInfo::new(
        "Test FX",
        "/clap/does-not-exist.clap",
        "org.example.fx",
        "/presets/does-not-exist",
    );
    let preset = |value: &str| AudioEffectPreset {
        plugin: plugin.key.clone(),
        json_key: plugin.json_key.clone(),
        value: value.to_string(),
        display: format!("Test FX: {value}"),
        path: std::path::PathBuf::from(format!("/presets/does-not-exist/{value}")),
    };
    let presets = vec![preset("Hall"), preset("Room")];
    AudioEffectCatalog::with_entries(vec![plugin], presets)
}

fn app_with_catalog() -> (DawApp, std::sync::mpsc::Receiver<crate::CacheJob>) {
    let (mut app, cache_rx) = build_test_app();
    app.plugin_entries =
        PluginEntries::none().with_effects(EffectPlugins::with_catalog(test_catalog()));
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] = INIT_WITH_PATCH.to_string();
    app.editor.data[2][1] = "cdef".to_string();
    (app, cache_rx)
}

fn press(app: &mut DawApp, keys: &[KeyCode]) {
    for &key in keys {
        match app.mode {
            DawMode::Normal => {
                app.handle_normal(key);
            }
            DawMode::EffectChain => app.handle_effect_chain(key),
            DawMode::EffectChainAdd => app.handle_effect_chain_add(key),
            other => panic!("unexpected mode {other:?} before {key:?}"),
        }
    }
}

fn init_json(app: &DawApp) -> serde_json::Value {
    let (value, _) = DawApp::extract_patch_json_and_phrase(&app.editor.data[2][0]).unwrap();
    value
}

#[test]
fn x_a_j_enter_enter_writes_a_one_element_chain_and_keeps_the_patch() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, cache_rx) = app_with_catalog();

    press(
        &mut app,
        &[
            KeyCode::Char('x'),
            KeyCode::Char('a'),
            KeyCode::Char('j'),
            KeyCode::Enter,
            KeyCode::Enter,
        ],
    );

    assert!(matches!(app.mode, DawMode::Normal));
    assert_eq!(
        init_json(&app),
        json!({
            "Surge XT patch": "Pads/Pad 1.fxp",
            "effects after instrument": [{"Test FX preset": "Room"}],
        })
    );
    // 依存セル（meas1）の cache が再 render に回る。
    let kicked: Vec<(usize, usize)> = cache_rx
        .try_iter()
        .map(|job| (job.track, job.measure))
        .collect();
    assert!(kicked.contains(&(2, 1)), "kicked: {kicked:?}");
}

#[test]
fn dd_deletes_the_stage_under_the_cursor_and_an_empty_chain_drops_the_key() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Hall"},{"Test FX preset":"Room"}]}"#.to_string();

    press(
        &mut app,
        &[
            KeyCode::Char('x'),
            KeyCode::Char('d'),
            KeyCode::Char('d'),
            KeyCode::Enter,
        ],
    );
    assert_eq!(
        init_json(&app),
        json!({
            "Surge XT patch": "Pads/Pad 1.fxp",
            "effects after instrument": [{"Test FX preset": "Room"}],
        })
    );

    press(
        &mut app,
        &[
            KeyCode::Char('x'),
            KeyCode::Char('d'),
            KeyCode::Char('d'),
            KeyCode::Enter,
        ],
    );
    assert_eq!(app.editor.data[2][0], INIT_WITH_PATCH);
}

#[test]
fn a_single_d_followed_by_another_key_does_not_delete() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Hall"}]}"#.to_string();
    let before = app.editor.data[2][0].clone();

    press(
        &mut app,
        &[
            KeyCode::Char('x'),
            KeyCode::Char('d'),
            KeyCode::Char('j'),
            KeyCode::Char('d'),
            KeyCode::Enter,
        ],
    );

    assert_eq!(app.editor.data[2][0], before);
}

#[test]
fn esc_discards_the_edit_and_keeps_the_init_cell() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, cache_rx) = app_with_catalog();

    press(
        &mut app,
        &[
            KeyCode::Char('x'),
            KeyCode::Char('a'),
            KeyCode::Enter,
            KeyCode::Esc,
        ],
    );

    assert!(matches!(app.mode, DawMode::Normal));
    assert_eq!(app.editor.data[2][0], INIT_WITH_PATCH);
    assert_eq!(cache_rx.try_iter().count(), 0);
}

#[test]
fn esc_in_the_add_overlay_returns_to_the_chain_without_adding() {
    let (mut app, _cache_rx) = app_with_catalog();

    press(
        &mut app,
        &[KeyCode::Char('x'), KeyCode::Char('a'), KeyCode::Esc],
    );

    assert!(matches!(app.mode, DawMode::EffectChain));
    assert!(app.overlays.effect_chain.chain.is_empty());
}

#[test]
fn x_is_ignored_on_the_chord_row_and_the_conductor_row() {
    let (mut app, _cache_rx) = app_with_catalog();
    for track in [0, crate::CHORD_TRACK] {
        app.editor.cursor_track = track;

        app.handle_normal(KeyCode::Char('x'));

        assert!(matches!(app.mode, DawMode::Normal), "track {track}");
    }
}

#[test]
fn x_does_not_open_on_a_route_without_effects() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;

    app.handle_normal(KeyCode::Char('x'));

    assert!(matches!(app.mode, DawMode::Normal));
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some(crate::messages::effect_chain::NOT_AVAILABLE_ON_THIS_BACKEND)
    );
}

#[test]
fn a_with_an_empty_catalog_stays_in_the_chain_overlay() {
    let (mut app, _cache_rx) = build_test_app();
    app.plugin_entries = PluginEntries::none()
        .with_effects(EffectPlugins::with_catalog(AudioEffectCatalog::default()));
    app.editor.cursor_track = 2;

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);

    assert!(matches!(app.mode, DawMode::EffectChain));
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some(crate::messages::effect_chain::NO_PRESETS)
    );
}

#[test]
fn the_overlay_shows_the_instrument_and_the_existing_chain_in_order() {
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Room"},{"Unknown preset":"x"}]}"#.to_string();

    app.handle_normal(KeyCode::Char('x'));

    let state = &app.overlays.effect_chain;
    assert_eq!(state.track, 2);
    assert_eq!(state.instrument, "Pads/Pad 1.fxp");
    assert_eq!(
        state.chain,
        vec![
            json!({"Test FX preset": "Room"}),
            json!({"Unknown preset": "x"})
        ]
    );
    let catalog = app.plugin_entries.effects().catalog();
    assert_eq!(
        crate::overlays::effect_stage_label(&state.chain[0], catalog),
        "Test FX: Room"
    );
    assert_eq!(
        crate::overlays::effect_stage_label(&state.chain[1], catalog),
        r#"{"Unknown preset":"x"}"#
    );
}
