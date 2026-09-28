//! Chord Chart の試聴（`t` / `Shift+T`・degrees 編集 overlay）と演奏（section の preview）が、
//! 同じ auto reverb の chain を音色の準備に載せる。

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use cmrt_core::{
    AudioEffectCatalog, AudioEffectPluginInfo, AudioEffectPreset, AudioPluginInfo, EffectPlugins,
};
use cmrt_mml_overlay::{LivePatch, MmlOverlaySender, RecordingSink};
use cmrt_patch_select::auto_reverb::AutoReverbRules;
use cmrt_runtime::{DEXED_PLUGIN_ID, SURGE_XT_PLUGIN_ID};
use cmrt_tui_core::patch_load::PatchCatalogSnapshot;
use cmrt_tui_core::patch_plugins::CatalogPlugin;

use super::chord_chart_preview::app_on_the_chord_chart;
use super::*;
use cmrt_chord_chart::PreviewRequest;

pub(crate) const DEXED_SNARE: &str = "Drums.syx/01 Snare Tight";
const DEXED_PAD: &str = "Factory.syx/10 Warm Pad";
const SURGE_PAD: &str = "Pads/Pad 1.fxp";
const DRUM_ROOM_CHAIN: &str = r#"[{"Dragonfly Room Reverb preset":"Small Drum Room"}]"#;
const PAD_HALL_CHAIN: &str = r#"[{"Dragonfly Hall Reverb preset":"Dark Room"}]"#;

/// Dexed（effect 無し）の snare と pad、Surge XT（effect 内蔵）の pad を持つ patch catalog。
pub(crate) fn patch_load() -> PatchLoadState {
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
    let dexed = audio_info("Dexed", DEXED_PLUGIN_ID);
    let surge = audio_info("Surge XT", SURGE_XT_PLUGIN_ID);
    let patches = vec![
        dexed.describe_patch(DEXED_SNARE, None),
        dexed.describe_patch(DEXED_PAD, None),
        surge.describe_patch(SURGE_PAD, None),
    ];
    let pairs = patches
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
        patches,
        vec![
            catalog_plugin("Dexed", DEXED_PLUGIN_ID),
            catalog_plugin("Surge XT", SURGE_XT_PLUGIN_ID),
        ],
        Vec::new(),
        BTreeMap::new(),
    )))
}

/// マシンに依存しない effect catalog。既定のルールが使う Dragonfly の 2 つだけ。
pub(crate) fn effect_plugins() -> EffectPlugins {
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
        preset(&hall, "Dark Room"),
    ];
    EffectPlugins::with_catalog(AudioEffectCatalog::with_entries(vec![room, hall], presets))
}

/// 保存先を一時 dir にした Chord Chart。保存済みのルールは無い（既定のルール）。
fn app_with_catalogs(tag: &str) -> (cmrt_history::test_support::LocalDirGuards, TuiApp<'static>) {
    let dirs = cmrt_history::test_support::temp_local_dirs(tag);
    let mut app = app_on_the_chord_chart();
    app.effect_plugins = effect_plugins();
    *app.patch_load_state.lock().unwrap() = patch_load();
    (dirs, app)
}

fn attach_recording_sink(app: &mut TuiApp<'_>) -> Arc<RecordingSink> {
    let sink = Arc::new(RecordingSink::default());
    app.mml_overlay_sender = Some(MmlOverlaySender::with_recording_sink(
        Arc::clone(&sink),
        48_000.0,
    ));
    sink
}

fn wait_for_prepared(sink: &RecordingSink, count: usize) -> Vec<LivePatch> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while sink.prepared().len() < count {
        assert!(
            Instant::now() < deadline,
            "音色の準備が {count} 件届かない: {:?}",
            sink.prepared()
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    sink.prepared()
}

fn section_request() -> PreviewRequest {
    PreviewRequest::section("A", "I-V-VIm-IV", None)
}

/// 演奏の layer が server へ準備させる音色と chain（instance ごと）。
fn playback_patch(app: &TuiApp<'_>, instance: u8) -> LivePatch {
    let preview = app.chord_chart_preview(&section_request());
    let layer = preview
        .layers
        .iter()
        .find(|layer| layer.instance_id == instance)
        .unwrap_or_else(|| panic!("instance {instance} layer が無い"));
    LivePatch::with_effect_chain(layer.patch.as_deref(), &layer.effect_chain)
}

#[test]
fn the_chord_and_bass_layers_each_get_the_chain_of_their_own_patch() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_auto_reverb_layers");
    app.chord_chart.set_bass_enabled(true);
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());
    app.chord_chart_bass_patch = Some(DEXED_PAD.to_string());

    assert_eq!(
        playback_patch(&app, 0),
        LivePatch::with_effect_chain(Some(DEXED_SNARE), DRUM_ROOM_CHAIN)
    );
    assert_eq!(
        playback_patch(&app, 1),
        LivePatch::with_effect_chain(Some(DEXED_PAD), PAD_HALL_CHAIN)
    );
}

/// 実際の演奏経路（カーソル移動 → `play_layers`）で、sender が 2 つの layer を chain 付きで準備する。
#[test]
fn playing_a_section_prepares_both_layers_with_their_chains() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_auto_reverb_play");
    app.chord_chart.set_bass_enabled(true);
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());
    app.chord_chart_bass_patch = Some(DEXED_PAD.to_string());
    let sink = attach_recording_sink(&mut app);

    app.handle_chord_chart_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));

    assert_eq!(
        wait_for_prepared(&sink, 2),
        vec![
            LivePatch::with_effect_chain(Some(DEXED_SNARE), DRUM_ROOM_CHAIN),
            LivePatch::with_effect_chain(Some(DEXED_PAD), PAD_HALL_CHAIN),
        ]
    );
}

#[test]
fn a_surge_patch_has_builtin_effects_and_gets_no_chain() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_auto_reverb_surge");
    app.chord_chart_patch = Some(SURGE_PAD.to_string());

    assert_eq!(playback_patch(&app, 0), LivePatch::new(Some(SURGE_PAD)));
}

#[test]
fn off_removes_the_chain_and_stays_off_after_entering_again() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_auto_reverb_off");
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());
    let mut rules = AutoReverbRules::default();
    rules.set_enabled(false);

    app.set_chord_chart_auto_reverb_rules(rules);
    assert_eq!(playback_patch(&app, 0), LivePatch::new(Some(DEXED_SNARE)));

    // 保存したルールを、画面へ入り直したときに読み直す。
    app.enter_chord_chart();
    assert_eq!(playback_patch(&app, 0), LivePatch::new(Some(DEXED_SNARE)));
}

#[test]
fn without_a_patch_catalog_nothing_is_added() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_auto_reverb_loading");
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());
    *app.patch_load_state.lock().unwrap() = PatchLoadState::Loading;

    assert_eq!(playback_patch(&app, 0), LivePatch::new(Some(DEXED_SNARE)));
}

#[test]
fn the_t_audition_prepares_the_same_chain_as_the_playback() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_auto_reverb_t");
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());
    let sink = attach_recording_sink(&mut app);

    assert!(app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)));

    let audition = wait_for_prepared(&sink, 1)[0].clone();
    assert_eq!(
        audition,
        LivePatch::with_effect_chain(Some(DEXED_SNARE), DRUM_ROOM_CHAIN)
    );
    assert_eq!(audition, playback_patch(&app, 0));
}

#[test]
fn the_shift_t_audition_prepares_the_same_chain_as_the_bass_playback() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_auto_reverb_shift_t");
    app.chord_chart.set_bass_enabled(true);
    app.chord_chart_patch = Some(SURGE_PAD.to_string());
    app.chord_chart_bass_patch = Some(DEXED_PAD.to_string());
    let sink = attach_recording_sink(&mut app);

    assert!(app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT)));

    let audition = wait_for_prepared(&sink, 1)[0].clone();
    assert_eq!(
        audition,
        LivePatch::with_effect_chain(Some(DEXED_PAD), PAD_HALL_CHAIN)
    );
    assert_eq!(audition, playback_patch(&app, 1));
}

#[test]
fn toggling_off_in_the_t_selector_makes_the_audition_and_the_playback_dry() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_auto_reverb_toggle");
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());
    let sink = attach_recording_sink(&mut app);
    assert!(app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)));
    wait_for_prepared(&sink, 1);

    app.handle_chord_chart_patch_select_key_event(KeyEvent::new(
        KeyCode::Char('e'),
        KeyModifiers::NONE,
    ));

    // カーソルの候補（Chord role の Dexed pad）を chain 無しで鳴らし直す。
    let prepared = wait_for_prepared(&sink, 2);
    assert_eq!(prepared[1], LivePatch::new(Some(DEXED_PAD)));
    assert_eq!(playback_patch(&app, 0), LivePatch::new(Some(DEXED_SNARE)));
    // 保存もされている。入り直しても dry のまま。
    app.handle_chord_chart_patch_select_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.enter_chord_chart();
    assert_eq!(playback_patch(&app, 0), LivePatch::new(Some(DEXED_SNARE)));
}

#[test]
fn the_degrees_overlay_prepares_the_chord_chart_chain() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_auto_reverb_degrees");
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());
    let sink = attach_recording_sink(&mut app);
    let section_id = app.chord_chart.selected_section().expect("section").id;

    app.open_chord_chart_degrees_overlay(section_id);

    assert_eq!(
        wait_for_prepared(&sink, 1)[0],
        LivePatch::with_effect_chain(Some(DEXED_SNARE), DRUM_ROOM_CHAIN)
    );
}

/// auto reverb は Chord Chart と grid だけ。どの画面からでも開く `Ctrl+P` には掛けない。
#[test]
fn the_global_ctrl_p_overlay_gets_no_chain() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_auto_reverb_ctrl_p");
    app.mml_overlay_patch = Some(DEXED_SNARE.to_string());
    let sink = attach_recording_sink(&mut app);

    assert!(app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)));

    assert_eq!(
        wait_for_prepared(&sink, 1)[0],
        LivePatch::new(Some(DEXED_SNARE))
    );
}
