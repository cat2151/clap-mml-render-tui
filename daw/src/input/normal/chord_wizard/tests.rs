use std::collections::BTreeMap;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use cmrt_tui_core::patch_load::{PatchCatalogSnapshot, PatchLoadMeasurement, PatchLoadState};

use crate::input::tests::build_test_app;
use crate::{DawPlayState, CHORD_TRACK, DEFAULT_TRACK0_MML, FIRST_PLAYABLE_TRACK};

#[test]
fn random_patch_logs_its_catalog_load_estimate_before_preview() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = FIRST_PLAYABLE_TRACK;
    app.editor.cursor_measure = 1;
    app.chord_progression_source = Some(Arc::new(|| vec!["I-IV".to_string()]));

    let patch = "Pads/Nine Second Pad.fxp";
    let mut measurements = BTreeMap::new();
    measurements.insert(
        patch.to_string(),
        PatchLoadMeasurement {
            second_load_ms: Some(9_000),
            ..PatchLoadMeasurement::default()
        },
    );
    *app.patch_load.lock().unwrap() = PatchLoadState::Ready(Arc::new(PatchCatalogSnapshot::new(
        vec![(patch.to_string(), patch.to_lowercase())],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        measurements,
    )));

    app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));

    let logs = app.log_lines.lock().unwrap();
    assert!(logs.iter().any(|line| {
        line == "chord wizard: random patch load estimate=9.000s (catalog) patch=\"Pads/Nine Second Pad.fxp\""
    }));
}

#[test]
fn missing_catalog_measurement_is_logged_as_unknown() {
    assert_eq!(
        super::random_patch_load_estimate_log_line("Pads/Unknown.fxp", None),
        "chord wizard: random patch load estimate=unknown (catalog) patch=\"Pads/Unknown.fxp\""
    );
}

#[test]
fn realtime_preview_uses_the_first_chord_patch_and_context_without_offline_preview() {
    let (mut app, cache_rx) = build_test_app();
    let track = FIRST_PLAYABLE_TRACK;
    app.editor.cursor_track = track;
    app.editor.cursor_measure = 1;
    app.editor.data[0][0] = DEFAULT_TRACK0_MML.to_string();
    app.editor.data[CHORD_TRACK][0] = "key:G".to_string();

    app.apply_chord_wizard_with("I-IV", Some("Pads/Warm Pad.fxp".to_string()));

    let request = app
        .chord_wizard_realtime_preview(track, 1)
        .expect("wizard should produce a realtime preview");
    assert_eq!(request.patch.as_deref(), Some("Pads/Warm Pad.fxp"));
    assert!(!request.program.repeat);
    assert_eq!(request.program.filters, Default::default());
    let pitches: Vec<u8> = request
        .program
        .events()
        .iter()
        .filter(|event| event.message[0] == 0x90 && event.message[2] > 0)
        .map(|event| event.message[1])
        .collect();
    assert_eq!(pitches, [67, 71, 74]);

    // テスト app には realtime sender が無い。それでも従来の offline preview へ
    // fallback せず、cache render だけは従来どおり meas.1 を含めて投入する。
    assert!(*app.playback.play_state.lock().unwrap() == DawPlayState::Idle);
    assert!(cache_rx
        .try_iter()
        .any(|job| job.track == track && job.measure == 1));
    let logs = app.log_lines.lock().unwrap();
    assert!(!logs.iter().any(|line| line == "preview: meas1"));
    assert!(logs.iter().any(|line| {
        line == "chord wizard: realtime preview unavailable (play server is not initialized)"
    }));
}
