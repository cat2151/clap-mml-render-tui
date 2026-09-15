use super::*;
use crate::tui::tests::test_config;
use crate::tui::TuiApp;

fn make_patches(items: &[&str]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|item| ((*item).to_string(), item.to_lowercase()))
        .collect()
}

#[test]
fn saved_bass_patch_wins_without_waiting_for_the_catalog() {
    let mut app = TuiApp::new_for_test(test_config());
    app.chord_chart_patch = Some("Chord/Piano.fxp".to_string());
    app.chord_chart_bass_patch = Some("Bass/Saved Bass.fxp".to_string());
    *app.patch_load_state.lock().unwrap() = PatchLoadState::Loading;

    assert_eq!(
        app.resolve_chord_chart_bass_patch(),
        BassPatchResolution::Ready("Bass/Saved Bass.fxp".to_string())
    );
}

#[test]
fn first_bass_role_candidate_is_selected_deterministically() {
    let app = TuiApp::new_for_test(test_config());
    *app.patch_load_state.lock().unwrap() = PatchLoadState::ready(make_patches(&[
        "Keys/Piano.fxp",
        "Bass/First Bass.fxp",
        "Bass/Second Bass.fxp",
    ]));

    assert_eq!(
        app.resolve_chord_chart_bass_patch(),
        BassPatchResolution::Ready("Bass/First Bass.fxp".to_string())
    );
}

#[test]
fn unavailable_catalog_states_fall_back_without_copying_the_chord_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.chord_chart_patch = Some("Chord/Piano.fxp".to_string());

    for (state, expected) in [
        (PatchLoadState::Loading, BassPatchResolution::Loading),
        (
            PatchLoadState::Err("catalog failed".to_string()),
            BassPatchResolution::CatalogError("catalog failed".to_string()),
        ),
        (
            PatchLoadState::ready(make_patches(&["Chord/Piano.fxp"])),
            BassPatchResolution::NoCandidate,
        ),
    ] {
        *app.patch_load_state.lock().unwrap() = state;
        assert_eq!(app.resolve_chord_chart_bass_patch(), expected);
    }
}
