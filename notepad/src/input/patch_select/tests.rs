use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use cmrt_core::{
    AudioEffectCatalog, AudioEffectPluginInfo, AudioEffectPreset, AudioPluginInfo, EffectPlugins,
};
use cmrt_runtime::{DEXED_PLUGIN_ID, SURGE_XT_PLUGIN_ID};
use cmrt_tui_core::patch_load::PatchCatalogSnapshot;
use cmrt_tui_core::patch_plugins::CatalogPlugin;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{json, Value};

use crate::tests::test_config;
use crate::{Mode, NotepadScreen, PatchLoadState, PlayState};

const DEXED_SNARE: &str = "Drums.syx/01 Snare Tight";
const SURGE_PAD: &str = "Pads/Pad 1.fxp";
const EXISTING_CHAIN_LINE: &str = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Dragonfly Hall Reverb preset":"Medium Clear Hall"}]} l8cdef"#;

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

/// history を test 専用の空 dir に向け、スコープを抜けると戻す。
struct HistoryDirGuard(Option<std::path::PathBuf>);

impl HistoryDirGuard {
    fn isolated() -> Self {
        let dir = cmrt_history::test_support::unique_test_dir("notepad_auto_reverb");
        Self(cmrt_history::test_support::set_app_dir_for_current_thread(
            Some(dir),
        ))
    }
}

impl Drop for HistoryDirGuard {
    fn drop(&mut self) {
        cmrt_history::test_support::set_app_dir_for_current_thread(self.0.take());
    }
}

fn open_on(line: &str, patch: &str) -> NotepadScreen<'static> {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.effect_plugins = test_effect_plugins();
    app.editor.lines = vec![line.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(dexed_and_surge_state()));
    app.open_patch_select_overlay(Some(patch));
    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(
        app.patch_select_selected_patch_name().as_deref(),
        Some(patch)
    );
    app
}

fn playing(app: &NotepadScreen<'_>) -> String {
    match &*app.playback.session.play_state().lock().unwrap() {
        PlayState::Running(mml) => mml.clone(),
        _ => panic!("the selector is not playing"),
    }
}

fn press(app: &mut NotepadScreen<'_>, code: KeyCode, modifiers: KeyModifiers) {
    app.handle_patch_select(KeyEvent::new(code, modifiers));
}

/// 試聴 MML と、`Enter` で確定した行。
fn preview_and_confirm(mut app: NotepadScreen<'_>) -> (String, String) {
    let preview = playing(&app);
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(matches!(app.mode, Mode::Normal));
    (preview, app.editor.lines[0].clone())
}

fn head_json(mml: &str) -> Value {
    NotepadScreen::extract_patch_json_value(mml).expect("line has head JSON")
}

fn chain(mml: &str) -> Option<Value> {
    head_json(mml).get("effects after instrument").cloned()
}

#[test]
fn confirming_a_dexed_snare_writes_the_drum_room_chain_that_the_preview_played() {
    let _history = HistoryDirGuard::isolated();
    let app = open_on(
        &format!(r#"{{"Surge XT patch":"{SURGE_PAD}"}} l8cdef"#),
        DEXED_SNARE,
    );

    let (preview, line) = preview_and_confirm(app);

    assert_eq!(
        chain(&line),
        Some(json!([{"Dragonfly Room Reverb preset": "Small Drum Room"}]))
    );
    assert_eq!(head_json(&line)["Surge XT patch"], DEXED_SNARE);
    assert_eq!(preview, line);
}

#[test]
fn confirming_a_surge_patch_writes_no_chain() {
    let _history = HistoryDirGuard::isolated();
    let app = open_on(
        &format!(r#"{{"Surge XT patch":"{DEXED_SNARE}"}} l8cdef"#),
        SURGE_PAD,
    );

    let (preview, line) = preview_and_confirm(app);

    assert_eq!(chain(&line), None);
    assert_eq!(preview, line);
}

#[test]
fn a_line_with_a_chain_keeps_it_in_the_preview_and_after_confirming() {
    let _history = HistoryDirGuard::isolated();
    let app = open_on(EXISTING_CHAIN_LINE, DEXED_SNARE);
    let original = chain(EXISTING_CHAIN_LINE);

    let (preview, line) = preview_and_confirm(app);

    assert_eq!(chain(&line), original);
    assert_eq!(head_json(&line)["Surge XT patch"], DEXED_SNARE);
    assert_eq!(preview, line);
}

#[test]
fn saved_off_setting_writes_no_chain() {
    let _history = HistoryDirGuard::isolated();
    cmrt_history::save_auto_reverb_settings(&cmrt_history::AutoReverbSettings {
        enabled: false,
        rules: BTreeMap::new(),
    })
    .unwrap();
    let app = open_on(
        &format!(r#"{{"Surge XT patch":"{SURGE_PAD}"}} l8cdef"#),
        DEXED_SNARE,
    );

    let (preview, line) = preview_and_confirm(app);

    assert_eq!(chain(&line), None);
    assert_eq!(preview, line);
}

#[test]
fn shift_e_saves_off_and_replays_the_same_patch_dry() {
    let _history = HistoryDirGuard::isolated();
    let mut app = open_on(
        &format!(r#"{{"Surge XT patch":"{SURGE_PAD}"}} l8cdef"#),
        DEXED_SNARE,
    );
    assert!(chain(&playing(&app)).is_some());

    press(&mut app, KeyCode::Char('E'), KeyModifiers::SHIFT);

    assert_eq!(chain(&playing(&app)), None);
    assert_eq!(head_json(&playing(&app))["Surge XT patch"], DEXED_SNARE);
    let saved = cmrt_history::load_auto_reverb_settings().expect("saved by E");
    assert!(!saved.enabled);
}

#[test]
fn the_rules_overlay_takes_the_keys_notepad_otherwise_handles_itself() {
    let _history = HistoryDirGuard::isolated();
    let mut app = open_on(
        &format!(r#"{{"Surge XT patch":"{SURGE_PAD}"}} l8cdef"#),
        DEXED_SNARE,
    );

    press(&mut app, KeyCode::Char('e'), KeyModifiers::NONE);
    // overlay 中の `t`・`p`・`n` は notepad の画面遷移に使われず、selector に残る。
    for ch in ['t', 'p', 'n'] {
        press(&mut app, KeyCode::Char(ch), KeyModifiers::NONE);
    }

    assert!(matches!(app.mode, Mode::PatchSelect));
    let select = app.patch_select.as_ref().expect("still open");
    assert!(select.auto_reverb_overlay_open());
}
