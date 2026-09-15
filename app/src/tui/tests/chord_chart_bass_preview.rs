//! Chord Chart app glue が Chord/Bass を別 layer のまま同じ command へ渡す計画。

use super::chord_chart_preview::app_on_the_chord_chart;
use super::*;
use crate::tui::chord_chart_glue::{preview_command_log_line, preview_play_log_line};
use cmrt_chord_chart::{PreviewRequest, PreviewVoicingContext};
use cmrt_mml_overlay::{line_play::LinePerformance, LineLayer};

fn note_ons(performance: &LinePerformance) -> Vec<(f64, u8)> {
    performance
        .events
        .iter()
        .filter(|event| event.message[0] & 0xf0 == 0x90 && event.message[2] > 0)
        .map(|event| (event.seconds, event.message[1]))
        .collect()
}

fn layer(preview: &crate::tui::chord_chart_glue::ChordChartPreview, instance: u8) -> &LineLayer {
    preview
        .layers
        .iter()
        .find(|layer| layer.instance_id == instance)
        .unwrap_or_else(|| panic!("instance {instance} layer が無い"))
}

#[test]
fn bass_on_builds_two_isolated_layers_with_distinct_patches() {
    let mut app = app_on_the_chord_chart();
    app.chord_chart.set_bass_enabled(true);
    app.chord_chart_patch = Some("Keys/Stage Piano.fxp".to_string());
    app.chord_chart_bass_patch = Some("Bass/Finger Bass.fxp".to_string());

    let preview = app.chord_chart_preview(&PreviewRequest::section("A", "I-V-VIm-IV", None));

    assert_eq!(preview.layers.len(), 2);
    let chord = layer(&preview, 0);
    let bass = layer(&preview, 1);
    assert_eq!(chord.patch.as_deref(), Some("Keys/Stage Piano.fxp"));
    assert_eq!(bass.patch.as_deref(), Some("Bass/Finger Bass.fxp"));
    assert_eq!(chord.performance, preview.program.performance);

    let chord_notes = note_ons(&chord.performance);
    let bass_notes = note_ons(&bass.performance);
    assert_eq!(chord_notes.len(), 12, "Chord layer は3音×4和音");
    assert_eq!(bass_notes.len(), 4, "Bass layer は各 chord に1音だけ");
    for (seconds, bass_note) in bass_notes {
        assert!(
            !chord_notes.contains(&(seconds, bass_note)),
            "Bass note が Chord layer に混ざらないこと: {seconds}s note={bass_note}"
        );
    }
}

#[test]
fn bass_off_keeps_the_original_chord_only_plan_unchanged() {
    let mut app = app_on_the_chord_chart();
    app.chord_chart_patch = Some("Keys/Stage Piano.fxp".to_string());
    app.chord_chart_bass_patch = Some("Bass/Finger Bass.fxp".to_string());

    let preview = app.chord_chart_preview(&PreviewRequest::section("A", "I-V-VIm-IV", None));

    assert_eq!(preview.layers.len(), 1);
    let chord = layer(&preview, 0);
    assert_eq!(chord.patch, preview.patch);
    assert_eq!(chord.performance, preview.program.performance);
    assert!(preview.bass_patch.is_none());
    assert!(preview.bass_reason.is_none());
}

#[test]
fn a_single_arrangement_chord_uses_the_bass_from_the_same_selected_voicing() {
    let mut app = app_on_the_chord_chart();
    app.chord_chart.set_bass_enabled(true);
    app.chord_chart_bass_patch = Some("Bass/Finger Bass.fxp".to_string());
    let request = PreviewRequest {
        name: "B".to_string(),
        degrees: "V-I".to_string(),
        chord_index: Some(1),
        voicing_context: PreviewVoicingContext {
            progressions: vec!["I-IV".to_string(), "V-I".to_string()],
            selected: 1,
        },
    };
    let all = cmrt_chord::parse_chord_progression("Key:C I-IV-V-I").unwrap();
    let expected = cmrt_chord::auto_voice_with_key(all.chords(), all.key_pitch_class(), None)[3]
        .bass
        .expect("auto voicing は Bass note を持つ");

    let preview = app.chord_chart_preview(&request);

    assert_eq!(
        note_ons(&layer(&preview, 1).performance),
        vec![(0.0, expected)]
    );
}

#[test]
fn the_first_bass_catalog_candidate_is_used_without_copying_the_chord_patch() {
    let mut app = app_on_the_chord_chart();
    app.chord_chart.set_bass_enabled(true);
    app.chord_chart_patch = Some("Keys/Stage Piano.fxp".to_string());
    *app.patch_load_state.lock().unwrap() = PatchLoadState::ready(make_patches(&[
        "Keys/Stage Piano.fxp",
        "Bass/First Bass.fxp",
        "Bass/Second Bass.fxp",
    ]));

    let preview = app.chord_chart_preview(&PreviewRequest::section("A", "I-V", None));

    assert_eq!(
        layer(&preview, 1).patch.as_deref(),
        Some("Bass/First Bass.fxp")
    );
    assert_ne!(layer(&preview, 0).patch, layer(&preview, 1).patch);
}

#[test]
fn unavailable_bass_catalog_states_preserve_the_chord_layer_with_a_reason() {
    for state in [
        PatchLoadState::Loading,
        PatchLoadState::Err("catalog failed".to_string()),
        PatchLoadState::ready(make_patches(&["Keys/Piano.fxp"])),
    ] {
        let mut app = app_on_the_chord_chart();
        app.chord_chart.set_bass_enabled(true);
        *app.patch_load_state.lock().unwrap() = state;

        let preview = app.chord_chart_preview(&PreviewRequest::section("A", "I-V", None));

        assert_eq!(preview.layers.len(), 1);
        assert_eq!(preview.layers[0].instance_id, 0);
        assert!(preview.bass_reason.is_some());
    }
}

#[test]
fn the_initial_preview_waits_for_the_bass_catalog_and_retries_automatically() {
    let mut app = TuiApp::new_for_test(test_config());
    app.chord_chart.set_bass_enabled(true);
    *app.patch_load_state.lock().unwrap() = PatchLoadState::Loading;

    app.switch_to_primary_screen(crate::screen_switch::PrimaryScreen::ChordChart, None);

    assert_eq!(
        app.deferred_chord_chart_preview
            .as_ref()
            .map(|request| request.name.as_str()),
        Some("A")
    );
    assert_eq!(
        app.chord_chart.error.as_deref(),
        Some("Bass patch catalog を読み込み中です")
    );

    *app.patch_load_state.lock().unwrap() = PatchLoadState::ready(make_patches(&[
        "Keys/Stage Piano.fxp",
        "Bass/First Bass.fxp",
    ]));
    app.pump_chord_chart_preview();

    assert!(app.deferred_chord_chart_preview.is_none());
    assert_eq!(app.chord_chart.error, None);
}

#[test]
fn parse_failure_is_silent_even_when_the_prefix_contains_note_letters() {
    let mut app = app_on_the_chord_chart();
    app.chord_chart.set_bass_enabled(true);
    app.chord_chart_bass_patch = Some("Bass/Finger Bass.fxp".to_string());

    let preview = app.chord_chart_preview(&PreviewRequest::section("broken", "zzz", None));

    assert!(preview.program.is_silent());
    assert!(preview.layers.is_empty());
}

#[test]
fn diagnostics_name_bass_state_patches_note_counts_and_command_id() {
    let mut app = app_on_the_chord_chart();
    app.chord_chart.set_bass_enabled(true);
    app.chord_chart_patch = Some("Keys/Stage Piano.fxp".to_string());
    app.chord_chart_bass_patch = Some("Bass/Finger Bass.fxp".to_string());
    let preview = app.chord_chart_preview(&PreviewRequest::section("A", "I-V", None));

    let play = preview_play_log_line(&preview);
    assert!(play.contains("bass=on"), "{play}");
    assert!(
        play.contains("chord_patch=\"Keys/Stage Piano.fxp\""),
        "{play}"
    );
    assert!(play.contains("chord_notes=6"), "{play}");
    assert!(
        play.contains("bass_patch=\"Bass/Finger Bass.fxp\""),
        "{play}"
    );
    assert!(play.contains("bass_notes=2"), "{play}");
    assert!(play.contains("bass_note_numbers=[48, 55]"), "{play}");
    assert!(play.contains("bass_note_range=48..=55(span=7)"), "{play}");
    assert_eq!(
        preview_command_log_line(&preview, Some(42)),
        "chord-chart: event=preview-command command_id=42 status=queued layers=2"
    );
}
