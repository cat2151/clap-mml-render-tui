//! Chord Chart の degrees editor と共有 MML overlay の owner 境界。

use super::*;
use crate::screen_switch::PrimaryScreen;
use crate::tui::mml_overlay::{MmlOverlayInputMode, MmlOverlaySyntax, SingleLineFlow};
use cmrt_history::test_support::temp_local_dirs;

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(code: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(code), KeyModifiers::CONTROL)
}

fn open_degrees(app: &mut TuiApp<'_>) -> cmrt_chord_chart::SectionId {
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    let section_id = app.chord_chart.selected_section().unwrap().id;
    assert_eq!(
        app.handle_chord_chart_key_event(plain(KeyCode::Char('i'))),
        cmrt_chord_chart::ChordChartAction::EditDegrees(section_id)
    );
    section_id
}

#[test]
fn i_opens_the_selected_section_in_modal_chord_chart_context() {
    let mut app = TuiApp::new_for_test(test_config());
    app.chord_chart.song.prefix = "BPM90 Key=F TEMPO=110".to_string();
    app.chord_chart_patch = Some("Chord/Piano.fxp".to_string());
    let expected = app.chord_chart.selected_section().unwrap().degrees.clone();

    let section_id = open_degrees(&mut app);

    assert!(app.mml_overlay.is_open());
    assert_eq!(
        app.mml_overlay_owner,
        Some(MmlOverlayOwner::ChordChart { section_id })
    );
    assert_eq!(
        app.mml_overlay.input_mode(),
        MmlOverlayInputMode::SingleLine
    );
    assert_eq!(app.mml_overlay.single_line_flow(), SingleLineFlow::Modal);
    assert_eq!(app.mml_overlay.value(), expected);
    assert_eq!(app.mml_overlay.patch(), Some("Chord/Piano.fxp"));
    assert_eq!(
        app.mml_overlay.syntax(),
        &MmlOverlaySyntax::ChordChart(cmrt_mml_overlay::ChordChartPreviewContext {
            key_token: Some("Key=F".to_string()),
        })
    );
}

#[test]
fn modal_enter_trims_saves_and_refreshes_ranges_immediately() {
    let (_tmp, _env_guard) = temp_local_dirs("chord_overlay_commit");
    let mut app = TuiApp::new_for_test(test_config());
    let section_id = open_degrees(&mut app);
    assert_eq!(app.chord_chart.chord_count(section_id), Some(4));

    for code in [KeyCode::Char('-'), KeyCode::Char('I'), KeyCode::Char(' ')] {
        app.handle_mml_overlay_key_event(plain(code));
    }
    app.handle_mml_overlay_key_event(plain(KeyCode::Enter));

    assert!(!app.mml_overlay.is_open());
    assert_eq!(app.mml_overlay_owner, None);
    assert_eq!(
        app.chord_chart.song.section(section_id).unwrap().degrees,
        "I-V-VIm-IV-I"
    );
    assert_eq!(app.chord_chart.chord_count(section_id), Some(5));
    assert_eq!(
        cmrt_chord_chart::load_song()
            .unwrap()
            .section(section_id)
            .unwrap()
            .degrees,
        "I-V-VIm-IV-I"
    );
}

#[test]
fn unchanged_enter_closes_without_writing_the_song() {
    let (_tmp, _env_guard) = temp_local_dirs("chord_overlay_unchanged");
    let mut app = TuiApp::new_for_test(test_config());
    open_degrees(&mut app);

    app.handle_mml_overlay_key_event(plain(KeyCode::Enter));

    assert!(!app.mml_overlay.is_open());
    assert_eq!(cmrt_chord_chart::load_song(), None);
}

#[test]
fn escape_discards_degrees_but_keeps_a_confirmed_chord_chart_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mml_overlay_patch = Some("Global/Pad.fxp".to_string());
    app.chord_chart_patch = Some("Chord/Old.fxp".to_string());
    *app.patch_load_state.lock().unwrap() = PatchLoadState::ready(make_patches(&["Chord/New.fxp"]));
    let original = app.chord_chart.selected_section().unwrap().degrees.clone();

    open_degrees(&mut app);
    app.handle_mml_overlay_key_event(ctrl('t'));
    app.handle_mml_overlay_key_event(plain(KeyCode::Enter));
    assert_eq!(app.chord_chart_patch.as_deref(), Some("Chord/New.fxp"));
    assert_eq!(app.mml_overlay_patch.as_deref(), Some("Global/Pad.fxp"));
    app.handle_mml_overlay_key_event(plain(KeyCode::Char('X')));
    app.handle_mml_overlay_key_event(plain(KeyCode::Esc));

    assert_eq!(
        app.chord_chart.selected_section().unwrap().degrees,
        original
    );
    assert_eq!(app.chord_chart_patch.as_deref(), Some("Chord/New.fxp"));
    assert_eq!(app.mml_overlay.patch(), Some("Global/Pad.fxp"));

    assert!(app.try_open_mml_overlay(ctrl('p')));
    assert_eq!(app.mml_overlay_owner, Some(MmlOverlayOwner::Global));
    assert_eq!(app.mml_overlay.patch(), Some("Global/Pad.fxp"));
}

#[test]
fn chord_chart_patch_filter_is_committed_before_the_patch_itself() {
    let mut app = TuiApp::new_for_test(test_config());
    *app.patch_load_state.lock().unwrap() =
        PatchLoadState::ready(make_patches(&["Chord/Lead.fxp", "Chord/Pad.fxp"]));
    open_degrees(&mut app);
    app.handle_mml_overlay_key_event(ctrl('t'));

    app.handle_mml_overlay_key_event(plain(KeyCode::Char('/')));
    for ch in "pad".chars() {
        app.handle_mml_overlay_key_event(plain(KeyCode::Char(ch)));
    }
    app.handle_mml_overlay_key_event(plain(KeyCode::Enter));

    assert!(app.mml_overlay.is_patch_select_open());
    assert_eq!(app.chord_chart_patch, None);

    app.handle_mml_overlay_key_event(plain(KeyCode::Enter));

    assert!(!app.mml_overlay.is_patch_select_open());
    assert_eq!(app.chord_chart_patch.as_deref(), Some("Chord/Pad.fxp"));
    assert!(app.mml_overlay.is_open());
}

#[test]
fn global_patch_confirmation_does_not_change_the_chord_chart_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.chord_chart_patch = Some("Chord/Piano.fxp".to_string());
    *app.patch_load_state.lock().unwrap() =
        PatchLoadState::ready(make_patches(&["Global/Lead.fxp"]));

    assert!(app.try_open_mml_overlay(ctrl('p')));
    app.handle_mml_overlay_key_event(ctrl('t'));
    app.handle_mml_overlay_key_event(plain(KeyCode::Enter));

    assert_eq!(app.mml_overlay_patch.as_deref(), Some("Global/Lead.fxp"));
    assert_eq!(app.chord_chart_patch.as_deref(), Some("Chord/Piano.fxp"));
}

#[test]
fn stale_section_commit_closes_without_writing_another_section() {
    let (_tmp, _env_guard) = temp_local_dirs("chord_overlay_stale");
    let mut app = TuiApp::new_for_test(test_config());
    let section_id = open_degrees(&mut app);
    app.chord_chart.song.sections.clear();

    app.handle_mml_overlay_key_event(plain(KeyCode::Char('X')));
    app.handle_mml_overlay_key_event(plain(KeyCode::Enter));

    assert!(app.chord_chart.song.section(section_id).is_none());
    assert!(!app.mml_overlay.is_open());
    assert_eq!(app.mml_overlay_owner, None);
    assert_eq!(cmrt_chord_chart::load_song(), None);
}

#[test]
fn catalog_loading_error_and_empty_states_are_safe_without_a_sender() {
    for state in [
        PatchLoadState::Loading,
        PatchLoadState::Err("catalog failed".to_string()),
        PatchLoadState::ready(Vec::new()),
    ] {
        let mut app = TuiApp::new_for_test(test_config());
        *app.patch_load_state.lock().unwrap() = state;
        open_degrees(&mut app);

        app.handle_mml_overlay_key_event(ctrl('t'));

        assert!(!app.mml_overlay.is_patch_select_open());
        assert!(app.mml_overlay.is_open());
        app.handle_mml_overlay_key_event(plain(KeyCode::Esc));
        assert!(!app.mml_overlay.is_open());
        assert_eq!(app.mml_overlay_owner, None);
    }
}

#[test]
fn history_round_trip_restores_the_two_canonical_patches_independently() {
    let (_tmp, _env_guard) = temp_local_dirs("chord_overlay_history");
    let cfg = test_config();
    let mut app = TuiApp::new_for_test(cfg.clone());

    *app.patch_load_state.lock().unwrap() =
        PatchLoadState::ready(make_patches(&["Chord/Piano.fxp"]));
    open_degrees(&mut app);
    app.handle_mml_overlay_key_event(ctrl('t'));
    app.handle_mml_overlay_key_event(plain(KeyCode::Enter));
    app.handle_mml_overlay_key_event(plain(KeyCode::Esc));

    *app.patch_load_state.lock().unwrap() =
        PatchLoadState::ready(make_patches(&["Global/Pad.fxp"]));
    assert!(app.try_open_mml_overlay(ctrl('p')));
    app.handle_mml_overlay_key_event(ctrl('t'));
    app.handle_mml_overlay_key_event(plain(KeyCode::Enter));
    app.handle_mml_overlay_key_event(plain(KeyCode::Esc));

    app.save_history_state();
    drop(app);

    let saved = crate::history::load_session_state();
    assert_eq!(saved.mml_overlay_patch.as_deref(), Some("Global/Pad.fxp"));
    assert_eq!(saved.chord_chart_patch.as_deref(), Some("Chord/Piano.fxp"));

    let restored = TuiApp::new(&cfg, cmrt_offline_render::PluginEntries::none());
    assert_eq!(
        restored.mml_overlay_patch.as_deref(),
        Some("Global/Pad.fxp")
    );
    assert_eq!(
        restored.chord_chart_patch.as_deref(),
        Some("Chord/Piano.fxp")
    );
    assert_eq!(restored.mml_overlay.patch(), Some("Global/Pad.fxp"));
    assert_eq!(restored.mml_overlay_owner, None);
}
