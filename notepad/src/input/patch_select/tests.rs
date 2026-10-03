use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use cmrt_core::AudioPluginInfo;
use cmrt_runtime::{DEXED_PLUGIN_ID, SURGE_XT_PLUGIN_ID};
use cmrt_tui_core::patch_load::PatchCatalogSnapshot;
use cmrt_tui_core::patch_plugins::CatalogPlugin;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{json, Value};

use crate::tests::{test_config, test_effect_plugins};
use crate::{Mode, NotepadScreen, PatchLoadState, PlayState};

const DEXED_SNARE: &str = "Drums.syx/01 Snare Tight";
const SURGE_PAD: &str = "Pads/Pad 1.fxp";
const EXISTING_CHAIN_LINE: &str = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Dragonfly Hall Reverb preset":"Medium Clear Hall"}]} l8cdef"#;

/// Dexed の snare（effect 無し）と Surge XT の pad（effect 内蔵）の 2 音色。
fn dexed_and_surge_state() -> PatchLoadState {
    let catalog_plugin = |name: &str, plugin_id: &str| CatalogPlugin {
        name: name.to_string(),
        plugin_path: format!("/clap/{name}.clap"),
        plugin_id: Some(plugin_id.to_string()),
        base: cmrt_runtime::PatchBase::None,
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
fn a_reverb_without_the_record_is_marked_manual_and_kept() {
    let _history = HistoryDirGuard::isolated();
    let app = open_on(EXISTING_CHAIN_LINE, DEXED_SNARE);
    let original = chain(EXISTING_CHAIN_LINE);
    assert_eq!(
        head_json(&app.editor.lines[0])["manual reverb"],
        json!(true)
    );

    let (preview, line) = preview_and_confirm(app);

    assert_eq!(chain(&line), original);
    assert_eq!(head_json(&line)["Surge XT patch"], DEXED_SNARE);
    assert_eq!(head_json(&line)["manual reverb"], json!(true));
    assert_eq!(preview, line);
}

/// delay の後ろに auto reverb が書いた Hall がある行。
fn delay_and_auto_hall_line() -> String {
    let hall = json!({"Dragonfly Hall Reverb preset": "Medium Clear Hall"});
    format!(
        "{} l8cdef",
        json!({
            "Surge XT patch": SURGE_PAD,
            "effects after instrument": [{"Delay preset": "Echo"}, hall.clone()],
            "auto reverb": hall,
        })
    )
}

#[test]
fn only_the_auto_reverb_stage_is_replaced_and_the_other_stages_stay() {
    let _history = HistoryDirGuard::isolated();
    let app = open_on(&delay_and_auto_hall_line(), DEXED_SNARE);
    let room = json!({"Dragonfly Room Reverb preset": "Small Drum Room"});

    let (preview, line) = preview_and_confirm(app);

    assert_eq!(
        chain(&line),
        Some(json!([{"Delay preset": "Echo"}, room.clone()]))
    );
    assert_eq!(head_json(&line)["auto reverb"], room);
    assert_eq!(preview, line);
}

#[test]
fn turning_auto_reverb_off_removes_only_its_stage_and_its_record() {
    let _history = HistoryDirGuard::isolated();
    let mut app = open_on(&delay_and_auto_hall_line(), DEXED_SNARE);
    press(&mut app, KeyCode::Char('e'), KeyModifiers::NONE);

    let (preview, line) = preview_and_confirm(app);

    assert_eq!(chain(&line), Some(json!([{"Delay preset": "Echo"}])));
    assert!(head_json(&line).get("auto reverb").is_none());
    assert!(head_json(&line).get("manual reverb").is_none());
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
fn e_saves_off_and_replays_the_same_patch_dry() {
    let _history = HistoryDirGuard::isolated();
    let mut app = open_on(
        &format!(r#"{{"Surge XT patch":"{SURGE_PAD}"}} l8cdef"#),
        DEXED_SNARE,
    );
    assert!(chain(&playing(&app)).is_some());

    press(&mut app, KeyCode::Char('e'), KeyModifiers::NONE);

    assert_eq!(chain(&playing(&app)), None);
    assert_eq!(head_json(&playing(&app))["Surge XT patch"], DEXED_SNARE);
    let saved = cmrt_history::load_auto_reverb_settings().expect("saved by e");
    assert!(!saved.enabled);
}

#[test]
fn the_rules_overlay_takes_the_keys_notepad_otherwise_handles_itself() {
    let _history = HistoryDirGuard::isolated();
    let mut app = open_on(
        &format!(r#"{{"Surge XT patch":"{SURGE_PAD}"}} l8cdef"#),
        DEXED_SNARE,
    );

    press(&mut app, KeyCode::Char('E'), KeyModifiers::SHIFT);
    // overlay 中の `t`・`p`・`n` は notepad の画面遷移に使われず、selector に残る。
    for ch in ['t', 'p', 'n'] {
        press(&mut app, KeyCode::Char(ch), KeyModifiers::NONE);
    }

    assert!(matches!(app.mode, Mode::PatchSelect));
    let select = app.patch_select.as_ref().expect("still open");
    assert!(select.auto_reverb_overlay_open());
}
