use std::sync::{Arc, Mutex};

use cmrt_drum_sequencer::{pattern_to_smf, DrumHit, DrumPattern, KitResolution, DEFAULT_VELOCITY};
use cmrt_mml_overlay::{MmlOverlaySender, RecordingSink};
use cmrt_tui_core::patch_load::{PatchCatalogSnapshot, PatchLoadMeasurement};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::screen_switch::PrimaryScreen;

fn temp_dir(label: &str) -> std::path::PathBuf {
    let unique = crate::tui::tests::NEXT_TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_drum_session_{label}_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    tmp
}

fn ready() -> PatchLoadState {
    let measurements = [
        ("Drums/Kit.sfz", vec![36, 38], Vec::new(), vec![36]),
        (
            "Kits/909.sfz",
            vec![38, 42],
            vec![(42, "Closed Hat".to_string())],
            Vec::new(),
        ),
    ]
    .into_iter()
    .map(|(name, notes, names, one_shot)| {
        (
            name.to_string(),
            PatchLoadMeasurement {
                drum_kit: true,
                drum_kit_notes: Some(notes),
                drum_kit_note_names: names,
                drum_kit_one_shot_notes: one_shot,
                ..Default::default()
            },
        )
    })
    .collect();
    let pairs = ["Drums/Kit.sfz", "Kits/909.sfz"]
        .into_iter()
        .map(|name| (name.to_string(), name.to_lowercase()))
        .collect();
    PatchLoadState::Ready(Arc::new(PatchCatalogSnapshot::new(
        pairs,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        measurements,
    )))
}

fn press(app: &mut TuiApp<'_>, code: KeyCode) {
    app.dispatch_drum_sequencer_key_event(KeyEvent::new(code, KeyModifiers::NONE));
}

/// selector を開き、一覧の先頭（Home）か末尾（End）の kit を確定する。
fn choose_kit(app: &mut TuiApp<'_>, edge: KeyCode) {
    for code in [KeyCode::Char('t'), edge, KeyCode::Enter] {
        press(app, code);
    }
    assert!(!app.drum_sequencer.selector_open());
}

fn saved_pattern_indexes(kit: &str) -> Vec<usize> {
    crate::history::load_drum_pattern_files(kit)
        .unwrap()
        .into_iter()
        .map(|(index, _)| index)
        .collect()
}

/// 起動時に走る catalog 読み込み thread と切り離し、テストが catalog の状態を決める。
fn restore_with_catalog(state: PatchLoadState) -> TuiApp<'static> {
    let cfg = Box::leak(Box::new(crate::tui::tests::test_config()));
    let mut app = TuiApp::new(cfg, cmrt_offline_render::EffectPlugins::none()).unwrap();
    app.patch_load_state = Arc::new(Mutex::new(state));
    app.mml_overlay_sender = Some(MmlOverlaySender::with_recording_sink(
        Arc::new(RecordingSink::default()),
        48_000.0,
    ));
    app
}

fn saved_drum_json() -> serde_json::Value {
    let saved = crate::history::load_session_state().unwrap();
    serde_json::to_value(&saved).unwrap()["drum_sequencer"].clone()
}

#[test]
fn drum_edits_are_kept_per_kit_and_pattern_and_return_after_the_catalog_is_ready() {
    let tmp = temp_dir("roundtrip");
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);
    let mut app = TuiApp::new_for_test(crate::tui::tests::test_config());
    *app.patch_load_state.lock().unwrap() = ready();
    app.switch_to_primary_screen(PrimaryScreen::DrumSequencer, None);
    choose_kit(&mut app, KeyCode::Home);
    assert_eq!(app.drum_sequencer.screen.kit_name(), Some("Drums/Kit.sfz"));
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Char('k'));
    for _ in 0..3 {
        press(&mut app, KeyCode::Char('l'));
    }
    press(&mut app, KeyCode::Char(' '));
    // 編集のたびに、kit の pattern ごとのファイルへ書く。
    assert_eq!(saved_pattern_indexes("Drums/Kit.sfz"), [0, 1]);

    // 909 は別の入力。編集中の pattern 番号とカーソルは持ち越す。
    choose_kit(&mut app, KeyCode::End);
    let screen = &app.drum_sequencer.screen;
    assert_eq!(screen.kit_name(), Some("Kits/909.sfz"));
    assert_eq!(screen.pattern_index(), 1);
    assert_eq!(screen.cursor_note(), Some(38));
    assert!(!screen.cell_on(38, 3));
    press(&mut app, KeyCode::Char('h'));
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(saved_pattern_indexes("Kits/909.sfz"), [1]);

    // 戻ると保存した入力が戻る。one-shot の 36 は 16 分音符、38 は 4 分音符。
    choose_kit(&mut app, KeyCode::Home);
    let screen = &app.drum_sequencer.screen;
    assert_eq!(screen.cell_length(38, 3), Some(4));
    assert!(!screen.cell_on(38, 2));
    assert_eq!(screen.pattern_at(0).unwrap().length(36, 0), Some(1));

    choose_kit(&mut app, KeyCode::End);
    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);
    app.save_history_state();
    drop(app);

    assert_eq!(
        saved_drum_json(),
        serde_json::json!({
            "kit": "Kits/909.sfz",
            "pattern": 1,
            "cursor_note": 38,
            "cursor_step": 2,
        })
    );

    let mut restored = restore_with_catalog(PatchLoadState::Loading);
    assert_eq!(restored.active_screen, PrimaryScreen::Notepad);
    // 読み込み待ちの間も、参照と入力を保ったまま待つ。
    restored.pump_mml_overlay();
    let screen = &restored.drum_sequencer.screen;
    assert_eq!(screen.kit_resolution(), Some(KitResolution::Waiting));
    assert_eq!(screen.kit_name(), Some("Kits/909.sfz"));
    assert_eq!(screen.pattern_index(), 1);
    assert!(screen.cell_on(38, 2));

    // 照合は drum 画面や selector を開いていなくても、毎フレームの同期で行う。
    *restored.patch_load_state.lock().unwrap() = ready();
    restored.pump_mml_overlay();
    let screen = &restored.drum_sequencer.screen;
    assert_eq!(screen.kit_resolution(), None);
    assert_eq!(screen.notes(), [38, 42]);
    assert_eq!(screen.note_name(42), Some("Closed Hat"));
    assert_eq!(screen.cursor_note(), Some(38));
    assert_eq!(screen.cursor_step(), 2);
    assert_eq!(screen.cell_length(38, 2), Some(4));
    assert!(!screen.cell_on(38, 3) && !screen.cell_on(42, 2));
    // 再生・試聴・selector・help は持ち越さない。
    assert_eq!(screen.playhead(), None);
    assert!(!screen.help_open());
    assert!(!restored.drum_sequencer.selector_open());
    assert!(restored.drum_sequencer.preview_command.is_none());
    assert!(restored.drum_sequencer.loop_hits.is_none());

    drop(restored);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn a_kit_missing_from_the_catalog_keeps_its_inputs_until_another_kit_is_chosen() {
    let tmp = temp_dir("missing_kit");
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);
    let hit = |step| DrumHit {
        note: 36,
        step,
        steps: 1,
        velocity: DEFAULT_VELOCITY,
    };
    let pattern = DrumPattern::from_hits([hit(0), hit(4)]);
    crate::history::save_drum_pattern_file(
        "Gone/Kit.sfz",
        0,
        Some(&pattern_to_smf("Gone/Kit.sfz", &pattern)),
    )
    .unwrap();
    crate::history::save_session_state(&crate::history::SessionState {
        active_screen: PrimaryScreen::DrumSequencer,
        drum_sequencer: Some(crate::history::DrumSequencerSessionState {
            kit: Some("Gone/Kit.sfz".to_string()),
            pattern: 0,
            cursor_note: Some(36),
            cursor_step: 4,
        }),
        mml_overlay_patch: Some("Leads/Lead.fxp".to_string()),
        ..Default::default()
    })
    .unwrap();

    let mut restored = restore_with_catalog(ready());
    assert_eq!(restored.active_screen, PrimaryScreen::DrumSequencer);
    restored.pump_mml_overlay();
    let screen = &restored.drum_sequencer.screen;
    assert_eq!(screen.kit_resolution(), Some(KitResolution::Missing));
    assert!(screen.notes().is_empty());
    assert!(screen.cell_on(36, 0) && screen.cell_on(36, 4));

    // 未解決のまま保存し直しても、参照と入力を消さない。
    restored.save_history_state();
    assert_eq!(saved_drum_json()["kit"], "Gone/Kit.sfz");
    assert_eq!(saved_pattern_indexes("Gone/Kit.sfz"), [0]);

    // 別の kit を選べる。入力はその kit のもの（空）に替わり、戻したカーソル note と step は
    // その kit の行に置かれる。
    press(&mut restored, KeyCode::Char('t'));
    assert!(restored.drum_sequencer.selector_open());
    press(&mut restored, KeyCode::Home);
    press(&mut restored, KeyCode::Enter);
    let screen = &restored.drum_sequencer.screen;
    assert_eq!(screen.kit_name(), Some("Drums/Kit.sfz"));
    assert_eq!(screen.kit_resolution(), None);
    assert_eq!(screen.cursor_note(), Some(36));
    assert_eq!(screen.cursor_step(), 4);
    assert!(screen.current_pattern().is_empty());
    assert_eq!(saved_pattern_indexes("Gone/Kit.sfz"), [0]);
    assert_eq!(
        restored.mml_overlay_patch.as_deref(),
        Some("Leads/Lead.fxp")
    );

    drop(restored);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn an_unreadable_catalog_leaves_the_saved_kit_unresolved() {
    let tmp = temp_dir("catalog_error");
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);
    let mut app = TuiApp::new_for_test(crate::tui::tests::test_config());
    app.drum_sequencer.screen.restore(
        Some("Drums/Kit.sfz".to_string()),
        [(
            0,
            DrumPattern::from_hits([DrumHit {
                note: 36,
                step: 0,
                steps: 1,
                velocity: DEFAULT_VELOCITY,
            }]),
        )],
        0,
        Some(36),
        0,
    );
    *app.patch_load_state.lock().unwrap() = PatchLoadState::Err("no cache".to_string());
    app.pump_mml_overlay();
    assert_eq!(
        app.drum_sequencer.screen.kit_resolution(),
        Some(KitResolution::Missing)
    );
    assert!(app.drum_sequencer.screen.cell_on(36, 0));
    drop(app);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn an_unedited_drum_screen_adds_nothing_and_a_broken_entry_keeps_other_screens() {
    let tmp = temp_dir("empty_and_broken");
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);
    let mut app = TuiApp::new_for_test(crate::tui::tests::test_config());
    app.mml_overlay_patch = Some("Leads/Lead.fxp".to_string());
    app.active_screen = PrimaryScreen::DrumSequencer;
    app.save_history_state();
    drop(app);
    assert_eq!(saved_drum_json(), serde_json::Value::Null);

    // drum 項目だけ壊れた history。
    let path = std::fs::read_dir(tmp.join("clap-mml-render-tui").join("history"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.file_name().is_some_and(|name| name == "history.json"))
        .expect("history.json");
    let mut json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    json["drum_sequencer"] = serde_json::json!({ "pattern": "broken", "cursor_step": -1 });
    std::fs::write(&path, json.to_string()).unwrap();

    let restored = restore_with_catalog(ready());
    assert_eq!(restored.active_screen, PrimaryScreen::DrumSequencer);
    assert_eq!(
        restored.mml_overlay_patch.as_deref(),
        Some("Leads/Lead.fxp")
    );
    assert_eq!(restored.drum_sequencer.screen.kit_name(), None);
    assert!(restored.drum_sequencer.screen.current_pattern().is_empty());
    drop(restored);
    std::fs::remove_dir_all(&tmp).ok();
}
