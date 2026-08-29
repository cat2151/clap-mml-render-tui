use std::collections::BTreeMap;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use cmrt_tui_core::patch_load::{PatchCatalogSnapshot, PatchLoadMeasurement, PatchLoadState};

use crate::input::tests::build_test_app;
use crate::FIRST_PLAYABLE_TRACK;

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
