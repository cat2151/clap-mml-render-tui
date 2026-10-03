//! Chord Chart が持つ Chord 音色の effect chain を、auto reverb と合成して鳴らす。

use cmrt_mml_overlay::LivePatch;
use cmrt_patches::PatchRole;
use serde_json::{json, Value};

use super::chord_chart_auto_reverb::{
    app_with_catalogs, attach_recording_sink, playback_patch, wait_for_prepared, DEXED_PAD,
    DEXED_SNARE, DRUM_ROOM_CHAIN, PAD_HALL_CHAIN,
};
use super::*;

/// effect catalog に無い（reverb とは判定されない）段。
fn delay_stage() -> Value {
    json!({"Example Delay preset": "Ping"})
}

/// effect catalog の reverb の段。
fn hall_stage() -> Value {
    json!({"Dragonfly Hall Reverb preset": "Dark Room"})
}

fn room_stage() -> Value {
    json!({"Dragonfly Room Reverb preset": "Small Drum Room"})
}

fn chain(stages: &[Value]) -> String {
    Value::Array(stages.to_vec()).to_string()
}

#[test]
fn without_a_manual_chain_the_chord_gets_only_the_auto_reverb() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_effect_chain_empty");
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());

    assert_eq!(
        app.chord_chart_effect_chain(PatchRole::Chord, Some(DEXED_SNARE)),
        DRUM_ROOM_CHAIN
    );
    assert_eq!(
        playback_patch(&app, 0),
        LivePatch::with_effect_chain(Some(DEXED_SNARE), DRUM_ROOM_CHAIN)
    );
}

#[test]
fn a_non_reverb_stage_is_followed_by_the_auto_reverb() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_effect_chain_delay");
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());
    app.chord_chart_effect_chain_stages = vec![delay_stage()];

    assert_eq!(
        playback_patch(&app, 0),
        LivePatch::with_effect_chain(Some(DEXED_SNARE), &chain(&[delay_stage(), room_stage()]))
    );
}

#[test]
fn a_manual_reverb_replaces_the_auto_reverb() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_effect_chain_manual_reverb");
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());
    app.chord_chart_effect_chain_stages = vec![delay_stage(), hall_stage()];

    assert_eq!(
        playback_patch(&app, 0),
        LivePatch::with_effect_chain(Some(DEXED_SNARE), &chain(&[delay_stage(), hall_stage()]))
    );
}

#[test]
fn the_bass_layer_does_not_get_the_manual_chain() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_effect_chain_bass");
    app.chord_chart.set_bass_enabled(true);
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());
    app.chord_chart_bass_patch = Some(DEXED_PAD.to_string());
    app.chord_chart_effect_chain_stages = vec![delay_stage()];

    assert_eq!(
        playback_patch(&app, 0),
        LivePatch::with_effect_chain(Some(DEXED_SNARE), &chain(&[delay_stage(), room_stage()]))
    );
    assert_eq!(
        playback_patch(&app, 1),
        LivePatch::with_effect_chain(Some(DEXED_PAD), PAD_HALL_CHAIN)
    );
}

/// patch catalog が無い間は auto reverb を足さないが、手動の chain はそのまま掛ける。
#[test]
fn without_a_patch_catalog_the_manual_chain_still_applies() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_effect_chain_loading");
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());
    app.chord_chart_effect_chain_stages = vec![delay_stage()];
    *app.patch_load_state.lock().unwrap() = PatchLoadState::Loading;

    assert_eq!(
        playback_patch(&app, 0),
        LivePatch::with_effect_chain(Some(DEXED_SNARE), &chain(&[delay_stage()]))
    );
}

fn app_with_delay_on_the_chord(
    tag: &str,
) -> (cmrt_history::test_support::LocalDirGuards, TuiApp<'static>) {
    let (dirs, mut app) = app_with_catalogs(tag);
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());
    app.chord_chart_effect_chain_stages = vec![delay_stage()];
    (dirs, app)
}

fn chord_with_delay_and_room() -> LivePatch {
    LivePatch::with_effect_chain(Some(DEXED_SNARE), &chain(&[delay_stage(), room_stage()]))
}

#[test]
fn the_t_audition_gets_the_manual_chain() {
    let (_dirs, mut app) = app_with_delay_on_the_chord("chord_chart_effect_chain_t");
    let sink = attach_recording_sink(&mut app);

    assert!(app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)));

    assert_eq!(wait_for_prepared(&sink, 1)[0], chord_with_delay_and_room());
}

#[test]
fn the_degrees_overlay_gets_the_manual_chain() {
    let (_dirs, mut app) = app_with_delay_on_the_chord("chord_chart_effect_chain_degrees");
    let sink = attach_recording_sink(&mut app);
    let section_id = app.chord_chart.selected_section().expect("section").id;

    app.open_chord_chart_degrees_overlay(section_id);

    assert_eq!(wait_for_prepared(&sink, 1)[0], chord_with_delay_and_room());
}

#[test]
fn the_shift_t_audition_does_not_get_the_manual_chain() {
    let (_dirs, mut app) = app_with_catalogs("chord_chart_effect_chain_shift_t");
    app.chord_chart.set_bass_enabled(true);
    app.chord_chart_bass_patch = Some(DEXED_PAD.to_string());
    app.chord_chart_effect_chain_stages = vec![delay_stage()];
    let sink = attach_recording_sink(&mut app);

    assert!(app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT)));

    assert_eq!(
        wait_for_prepared(&sink, 1)[0],
        LivePatch::with_effect_chain(Some(DEXED_PAD), PAD_HALL_CHAIN)
    );
}
