//! `t` の selector の auto reverb: 試聴の LivePatch と、確定後の init セルの chain。

use std::collections::BTreeMap;
use std::sync::Arc;

use cmrt_core::{
    AudioEffectCatalog, AudioEffectPluginInfo, AudioEffectPreset, AudioPluginInfo, EffectPlugins,
};
use cmrt_runtime::{DEXED_PLUGIN_ID, SURGE_XT_PLUGIN_ID};
use cmrt_tui_core::patch_load::{PatchCatalogSnapshot, PatchLoadState};
use cmrt_tui_core::patch_plugins::CatalogPlugin;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, text::Span, Terminal};
use serde_json::{json, Value};

use super::{attach_recording_sink, key, plain, wait_until};
use crate::input::tests::build_test_app;
use crate::mml::effect_chain::init_cell_effect_chain;
use crate::DawApp;

const DEXED_SNARE: &str = "Drums.syx/01 Snare Tight";
const SURGE_PAD: &str = "Pads/Pad 1.fxp";
const DRUM_ROOM_CHAIN: &str = r#"[{"Dragonfly Room Reverb preset":"Small Drum Room"}]"#;
const HALL_CHAIN: &str = r#"[{"Dragonfly Hall Reverb preset":"Medium Clear Hall"}]"#;

/// マシンに依存しない effect catalog。ルール既定値の 2 つの reverb だけを持つ。
fn test_effect_plugins() -> EffectPlugins {
    let room = AudioEffectPluginInfo::new(
        "Dragonfly Room Reverb",
        "/clap/room.clap",
        "org.example.room",
        "/clap/room.clap",
    );
    let hall = AudioEffectPluginInfo::new(
        "Dragonfly Hall Reverb",
        "/clap/hall.clap",
        "org.example.hall",
        "/clap/hall.clap",
    );
    let preset = |plugin: &AudioEffectPluginInfo, value: &str| AudioEffectPreset {
        plugin: plugin.key.clone(),
        json_key: plugin.json_key.clone(),
        value: value.to_string(),
        display: format!("{}: {value}", plugin.name),
        name: value.to_string(),
        category: "Space / Imaging".to_string(),
        kind: "Reverb".to_string(),
        path: std::path::PathBuf::from(format!("/presets/{value}")),
    };
    let presets = vec![
        preset(&room, "Small Drum Room"),
        preset(&hall, "Medium Clear Hall"),
    ];
    EffectPlugins::with_catalog(AudioEffectCatalog::with_entries(vec![room, hall], presets))
}

/// Dexed の snare（effect 無し）と Surge XT の pad（effect 内蔵）の 2 音色。
fn dexed_and_surge_state() -> PatchLoadState {
    let catalog_plugin = |name: &str, plugin_id: &str| CatalogPlugin {
        name: name.to_string(),
        plugin_path: format!("/clap/{name}.clap"),
        plugin_id: Some(plugin_id.to_string()),
        base: None,
        dirs: Vec::new(),
        resolved_patches: None,
        source_notices: Vec::new(),
    };
    let audio_info = |name: &str, plugin_id: &str| {
        AudioPluginInfo::new(
            name,
            format!("/clap/{name}.clap"),
            Some(plugin_id.to_string()),
            None,
        )
    };
    let dexed = audio_info("Dexed", DEXED_PLUGIN_ID).describe_patch(DEXED_SNARE, None);
    let surge = audio_info("Surge XT", SURGE_XT_PLUGIN_ID).describe_patch(SURGE_PAD, None);
    let pairs = [&dexed, &surge]
        .iter()
        .map(|patch| {
            (
                patch.reference.display.clone(),
                patch.normalized_display.clone(),
            )
        })
        .collect();
    PatchLoadState::Ready(Arc::new(PatchCatalogSnapshot::new(
        pairs,
        vec![dexed, surge],
        vec![
            catalog_plugin("Dexed", DEXED_PLUGIN_ID),
            catalog_plugin("Surge XT", SURGE_XT_PLUGIN_ID),
        ],
        Vec::new(),
        BTreeMap::new(),
    )))
}

fn patch_cell(patch: &str) -> String {
    format!(r#"{{"Surge XT patch": "{patch}"}}"#)
}

fn app_with_init_cell(init_cell: &str) -> (DawApp, std::sync::mpsc::Receiver<crate::CacheJob>) {
    let (mut app, cache_rx) = build_test_app();
    *app.patch_load.lock().unwrap() = dexed_and_surge_state();
    app.effect_plugins = test_effect_plugins();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] = init_cell.to_string();
    app.editor.data[2][1] = "c".to_string();
    (app, cache_rx)
}

/// `patch` の準備が届くまで待ち、届いた準備の chain を順に返す。
fn chains_prepared_for(sink: &cmrt_mml_overlay::RecordingSink, patch: &str) -> Vec<String> {
    wait_until(patch, || {
        sink.prepared()
            .iter()
            .any(|prepared| prepared.patch() == Some(patch))
    });
    sink.prepared()
        .iter()
        .filter(|prepared| prepared.patch() == Some(patch))
        .map(|prepared| prepared.effect_chain().to_string())
        .collect()
}

fn init_chain(app: &DawApp) -> Value {
    Value::Array(init_cell_effect_chain(&app.editor.data[2][0]))
}

fn head_json(app: &DawApp) -> Value {
    serde_json::Deserializer::from_str(&app.editor.data[2][0])
        .into_iter::<Value>()
        .next()
        .expect("init セルが JSON で始まる")
        .unwrap()
}

/// selector の画面。全角文字の後ろの継続セルは読み飛ばし、見えるとおりの並びにする。
fn render_selector(app: &DawApp) -> String {
    let select = app
        .direct_patch_select
        .as_ref()
        .expect("selector が開いている");
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal
        .draw(|frame| cmrt_patch_select::ui::draw_direct_patch_select(select, frame))
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            let mut line = String::new();
            let mut x = 0;
            while x < buffer.area.width {
                let symbol = buffer[(x, y)].symbol();
                line.push_str(symbol);
                x += (Span::raw(symbol).width() as u16).max(1);
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_dexed_snare_on_a_track_without_a_chain_previews_and_confirms_the_drum_room() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_init_cell(&patch_cell(DEXED_SNARE));
    let sink = attach_recording_sink(&mut app);

    app.handle_normal_key_event(plain('t'));

    assert_eq!(
        chains_prepared_for(&sink, DEXED_SNARE),
        vec![DRUM_ROOM_CHAIN]
    );
    assert!(render_selector(&app)
        .contains("auto reverb: Dragonfly Room Reverb: Small Drum Room (snare)"));

    // 音色は変えずに確定しても、掛かっていた reverb は chain へ書き、控えにも書く。
    app.handle_direct_patch_select_key_event(key(KeyCode::Enter));

    assert_eq!(
        init_chain(&app),
        json!([{"Dragonfly Room Reverb preset": "Small Drum Room"}])
    );
    assert_eq!(
        head_json(&app)["auto reverb"],
        json!({"Dragonfly Room Reverb preset": "Small Drum Room"})
    );
    assert_eq!(head_json(&app)["Surge XT patch"], DEXED_SNARE);
    assert_eq!(app.editor.data[2][1], "c");
}

#[test]
fn switching_to_a_dexed_snare_writes_the_patch_and_the_chain_together() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    // 音色キーの無い init セル。音色を書くときに JSON を作り直しても chain が残ること。
    let (mut app, _cache_rx) = app_with_init_cell(r#"{"memo": "x"}"#);

    app.handle_normal_key_event(plain('t'));
    app.handle_direct_patch_select_key_event(plain('/'));
    for ch in "snare".chars() {
        app.handle_direct_patch_select_key_event(plain(ch));
    }
    app.handle_direct_patch_select_key_event(key(KeyCode::Enter));
    app.handle_direct_patch_select_key_event(key(KeyCode::Enter));

    assert_eq!(head_json(&app)["Surge XT patch"], DEXED_SNARE);
    assert_eq!(
        init_chain(&app),
        json!([{"Dragonfly Room Reverb preset": "Small Drum Room"}])
    );
}

#[test]
fn a_reverb_without_the_record_is_marked_manual_and_kept() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let init_cell = format!(
        r#"{{"Surge XT patch": "{DEXED_SNARE}", "effects after instrument": {HALL_CHAIN}}}"#
    );
    let (mut app, _cache_rx) = app_with_init_cell(&init_cell);
    let sink = attach_recording_sink(&mut app);

    app.handle_normal_key_event(plain('t'));

    assert_eq!(chains_prepared_for(&sink, DEXED_SNARE), vec![HALL_CHAIN]);
    assert!(
        render_selector(&app).contains("manual reverb: Dragonfly Hall Reverb: Medium Clear Hall")
    );
    assert_eq!(head_json(&app)["manual reverb"], json!(true));

    app.handle_direct_patch_select_key_event(key(KeyCode::Enter));

    assert_eq!(
        init_chain(&app),
        serde_json::from_str::<Value>(HALL_CHAIN).unwrap()
    );
    assert_eq!(head_json(&app)["manual reverb"], json!(true));
    assert!(head_json(&app).get("auto reverb").is_none());
}

#[test]
fn only_the_auto_reverb_stage_is_replaced_and_the_other_stages_stay() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let delay = json!({"Delay preset": "Echo"});
    let hall = json!({"Dragonfly Hall Reverb preset": "Medium Clear Hall"});
    let room = json!({"Dragonfly Room Reverb preset": "Small Drum Room"});
    let init_cell = json!({
        "Surge XT patch": DEXED_SNARE,
        "effects after instrument": [delay.clone(), hall.clone()],
        "auto reverb": hall,
    })
    .to_string();
    let (mut app, _cache_rx) = app_with_init_cell(&init_cell);
    let sink = attach_recording_sink(&mut app);

    app.handle_normal_key_event(plain('t'));

    assert_eq!(
        chains_prepared_for(&sink, DEXED_SNARE),
        vec![Value::Array(vec![delay.clone(), room.clone()]).to_string()]
    );

    app.handle_direct_patch_select_key_event(key(KeyCode::Enter));

    assert_eq!(init_chain(&app), json!([delay, room.clone()]));
    assert_eq!(head_json(&app)["auto reverb"], room);
}

#[test]
fn a_surge_patch_previews_and_confirms_without_a_chain() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_init_cell(&patch_cell(SURGE_PAD));
    let sink = attach_recording_sink(&mut app);

    app.handle_normal_key_event(plain('t'));

    assert_eq!(chains_prepared_for(&sink, SURGE_PAD), vec![String::new()]);
    assert!(!render_selector(&app).contains("auto reverb"));

    app.handle_direct_patch_select_key_event(key(KeyCode::Enter));

    assert_eq!(head_json(&app)["Surge XT patch"], SURGE_PAD);
    assert_eq!(init_chain(&app), json!([]));
}

#[test]
fn e_saves_off_and_replays_the_same_patch_dry() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_init_cell(&patch_cell(DEXED_SNARE));
    let sink = attach_recording_sink(&mut app);
    app.handle_normal_key_event(plain('t'));
    assert_eq!(
        chains_prepared_for(&sink, DEXED_SNARE),
        vec![DRUM_ROOM_CHAIN]
    );

    app.handle_direct_patch_select_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));

    wait_until("off での鳴らし直し", || {
        chains_prepared_for(&sink, DEXED_SNARE).len() >= 2
    });
    assert_eq!(
        chains_prepared_for(&sink, DEXED_SNARE),
        vec![DRUM_ROOM_CHAIN.to_string(), String::new()]
    );
    let saved = cmrt_history::load_auto_reverb_settings().expect("off を保存した");
    assert!(!saved.enabled);
    assert!(render_selector(&app).contains("auto reverb: off"));

    // off のまま確定しても chain は書かない。
    app.handle_direct_patch_select_key_event(key(KeyCode::Enter));
    assert_eq!(init_chain(&app), json!([]));
}
