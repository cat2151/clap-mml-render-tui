//! Guitar Articulation 画面の履歴が、いつ専用 file へ書かれ、次の起動で読まれるか。
//! `Shift+H` の overlay の `j` で送られる演奏が、選んだ履歴の音になっているか。
//!
//! 保存先は `set_local_dir_envs` で temp へ隔離し、実 server の代わりに記録 sender を使う。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::*;
use crate::screen_switch::PrimaryScreen;
use crate::test_utils::TestEnvGuard;
use crate::tui::guitar_articulation::{convert, GuitarArticulationHistory, Rule, RuleTable};
use cmrt_mml_overlay::{MmlOverlaySender, RecordingSink};

const MML: &str = "o3 l8 e f+ g";

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn shift_h() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT)
}

/// 一時ディレクトリへ history を差し替える。戻り値を持っている間だけ有効。
fn isolated_history(tag: &str) -> (PathBuf, TestEnvGuard) {
    let tmp = crate::test_utils::unique_test_dir(tag);
    std::fs::remove_dir_all(&tmp).ok();
    let guard = crate::test_utils::set_local_dir_envs(&tmp);
    (tmp, guard)
}

fn saved_history() -> GuitarArticulationHistory {
    let path = crate::history::guitar_articulation_history_file_path().expect("保存先");
    let content = std::fs::read_to_string(&path).expect("履歴 file が書かれていること");
    serde_json::from_str(&content).expect("履歴 file を読めること")
}

/// 画面へ入り、MML 欄を `MML` に打ち替えて確定し、2 列目に `a`（H/P）を当てて matrix へ戻る。
fn app_with_mml_and_hammer<'a>() -> TuiApp<'a> {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);
    for code in [KeyCode::Char('a'), KeyCode::Char('k')] {
        app.handle_guitar_articulation_key_event(KeyEvent::new(code, KeyModifiers::CONTROL));
    }
    for ch in MML.chars() {
        app.handle_guitar_articulation_key_event(plain(KeyCode::Char(ch)));
    }
    app.handle_guitar_articulation_key_event(plain(KeyCode::Enter));
    for code in [KeyCode::Char('l'), KeyCode::Char('a'), KeyCode::Esc] {
        app.handle_guitar_articulation_key_event(plain(code));
    }
    app
}

fn hammer_rules() -> RuleTable {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::HammerPull);
    rules
}

fn messages(events: &[cmrt_chord::TimedMidiEvent]) -> Vec<[u8; 3]> {
    events.iter().map(|event| event.message).collect()
}

#[test]
fn opening_the_overlay_writes_the_file_with_the_state_just_before() {
    let (tmp, _guard) = isolated_history("ga_history_open");
    let mut app = app_with_mml_and_hammer();
    let path = crate::history::guitar_articulation_history_file_path().unwrap();
    assert!(!path.exists(), "開く前は書かれていない: {}", path.display());

    app.handle_guitar_articulation_key_event(shift_h());

    let saved = saved_history();
    let head = &saved.entries[0];
    assert_eq!(head.mml, MML);
    assert_eq!(head.rules, hammer_rules());
    assert_eq!(saved, *app.guitar_articulation.history());
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn j_in_the_overlay_plays_the_older_entry_once() {
    let (tmp, _guard) = isolated_history("ga_history_j");
    let mut app = app_with_mml_and_hammer();
    let sink = Arc::new(RecordingSink::default());
    app.mml_overlay_sender = Some(MmlOverlaySender::with_recording_sink(
        Arc::clone(&sink),
        48_000.0,
    ));
    let flat = cmrt_chord::timed_performance(MML).unwrap().events;
    let older = convert(&flat, &RuleTable::default());
    // 対照: 1 つ古い entry（H/P なし）は、開く直前（H/P あり）と違う音。
    assert_ne!(messages(&older), messages(&convert(&flat, &hammer_rules())));

    app.handle_guitar_articulation_key_event(shift_h());
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('j')));
    let deadline = Instant::now() + Duration::from_secs(5);
    while sink.timeline_events().len() < older.len() {
        assert!(Instant::now() < deadline, "試聴が積まれない");
        std::thread::sleep(Duration::from_millis(2));
    }
    std::thread::sleep(Duration::from_millis(50));

    let sent: Vec<[u8; 3]> = sink
        .timeline_events()
        .iter()
        .map(|event| event.message)
        .collect();
    assert_eq!(sent, messages(&older));
    assert_eq!(sink.timelines(), 1);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn history_saved_on_exit_is_loaded_by_the_next_app() {
    let (tmp, _guard) = isolated_history("ga_history_exit");
    let mut app = app_with_mml_and_hammer();
    let entries = app.guitar_articulation.history().clone();
    assert!(entries.entries.len() >= 2, "{entries:?}");

    app.save_notepad_and_session_state();
    let next = TuiApp::new_for_test(test_config());

    assert_eq!(*next.guitar_articulation.history(), entries);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn the_next_app_starts_from_the_state_at_exit() {
    let (tmp, _guard) = isolated_history("ga_history_restore");
    let mut app = app_with_mml_and_hammer();

    app.save_notepad_and_session_state();
    let mut next = TuiApp::new_for_test(test_config());
    next.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);

    assert_eq!(next.guitar_articulation.mml(), MML);
    assert_eq!(*next.guitar_articulation.rules(), hammer_rules());
    std::fs::remove_dir_all(&tmp).ok();
}
