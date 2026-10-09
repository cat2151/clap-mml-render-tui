use super::*;
use crate::screen_switch::PrimaryScreen;
use cmrt_history::test_support::{temp_local_dirs, LocalDirGuards};
use cmrt_mml_overlay::{MmlOverlaySender, RecordingSink};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
fn preview() -> KeyEvent {
    key(KeyCode::Char('P'))
}

fn wait_until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !ready() {
        assert!(Instant::now() < deadline, "sender condition timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// 編集のたびに kit の pattern ファイルを書くので、保存先はテストごとに分ける。
fn app_with_sink(sink: Arc<RecordingSink>) -> (LocalDirGuards, TuiApp<'static>) {
    let dirs = temp_local_dirs("drum_sequencer_preview");
    let mut app = TuiApp::new_for_test(crate::tui::tests::test_config());
    app.mml_overlay_sender = Some(MmlOverlaySender::with_recording_sink(sink, 48_000.0));
    app.switch_to_primary_screen(PrimaryScreen::DrumSequencer, None);
    app.drum_sequencer.screen.set_kit(
        "Kit.sfz".to_string(),
        Some(vec![36, 38]),
        Vec::new(),
        Vec::new(),
    );
    (dirs, app)
}

/// 停止中の Shift+P で繰り返し再生を始め、sender が演奏開始を公開するまで待つ。
fn play(app: &mut TuiApp<'_>, sink: &RecordingSink) {
    assert!(
        app.drum_sequencer.loop_hits.is_none(),
        "play starts from stopped"
    );
    let before = sink.timelines();
    app.dispatch_drum_sequencer_key_event(preview());
    wait_until(|| {
        sink.timelines() > before
            && app
                .mml_overlay_sender
                .as_ref()
                .unwrap()
                .status()
                .line_playback()
                .is_some()
    });
}

#[test]
fn loop_hits_are_kit_cells_in_step_order_and_keep_hidden_input_unsent() {
    let mut screen = DrumSequencerScreen::default();
    // 36 は one-shot。
    screen.set_kit(
        "kit".to_string(),
        Some(vec![36, 38, 42]),
        Vec::new(),
        vec![36],
    );
    screen.handle_key_event(key(KeyCode::Char(' ')));
    screen.handle_key_event(key(KeyCode::Char('k')));
    screen.handle_key_event(key(KeyCode::Char(' ')));
    for _ in 0..15 {
        screen.handle_key_event(key(KeyCode::Char('l')));
    }
    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Char(' ')));
    // 最後の 36 を 3 step に伸ばす。gate は周の終わりを越えてよい。
    for _ in 0..2 {
        screen.handle_key_event(key(KeyCode::Char('+')));
    }
    // velocity はセルごと。
    screen.handle_key_event(key(KeyCode::Char(',')));
    for _ in 0..2 {
        screen.handle_key_event(key(KeyCode::Char('k')));
    }
    screen.handle_key_event(key(KeyCode::Char(' ')));
    screen.set_kit(
        "kit subset".to_string(),
        Some(vec![36, 38]),
        Vec::new(),
        Vec::new(),
    );
    let hits: Vec<_> = loop_hits(&screen)
        .iter()
        .map(|hit| (hit.seconds, hit.note, hit.velocity, hit.gate_seconds))
        .collect();
    // gate は各セルの音長。one-shot でない 38 は 4 分音符。
    assert_eq!(
        hits,
        [
            (0.0, 36, 127, 0.125),
            (0.0, 38, 127, 0.5),
            (1.875, 36, 119, 0.375)
        ]
    );
    assert_eq!(
        LOOP_SECONDS, 2.0,
        "the cycle never follows the last hit or gate"
    );
    assert!(
        screen.cell_on(42, 15),
        "hidden input survives without being sent"
    );
    screen.set_kit("unknown".to_string(), None, Vec::new(), Vec::new());
    assert!(loop_hits(&screen).is_empty());
}

#[test]
fn loading_stops_old_queue_and_departure_cancels_pending_preview() {
    let sink = Arc::new(RecordingSink::default());
    let (_dirs, mut app) = app_with_sink(Arc::clone(&sink));
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Char(' ')));
    play(&mut app, &sink);
    let sender = app.mml_overlay_sender.as_ref().unwrap();
    sender.preload(LivePatch::new(Some("Background.sfz")));
    wait_until(|| !sink.preloads().is_empty());
    app.drum_sequencer.screen.set_kit(
        "NewKit.sfz".to_string(),
        Some(vec![36]),
        Vec::new(),
        Vec::new(),
    );
    let stops = sink.stops();
    // 1 回目で走っているループを止め、2 回目で新しい kit のループを始める。
    app.dispatch_drum_sequencer_key_event(preview());
    assert!(app.drum_sequencer.loop_hits.is_none());
    app.dispatch_drum_sequencer_key_event(preview());
    wait_until(|| {
        app.mml_overlay_sender
            .as_ref()
            .unwrap()
            .status()
            .is_loading()
    });
    assert!(
        sink.stops() > stops,
        "stop happens before async preparation"
    );
    assert_eq!(
        sink.timelines(),
        1,
        "no timeline is sent to old kit while loading"
    );
    let lines = crate::tui::ui::tests::render_lines(&mut app, 120, 30).join("\n");
    assert!(lines
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .contains("音源"));
    assert!(app.sound_startup_wait.is_some());
    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);
    sink.finish_preload();
    wait_until(|| {
        app.mml_overlay_sender
            .as_ref()
            .unwrap()
            .status()
            .line_playback()
            .is_none()
            && !app
                .mml_overlay_sender
                .as_ref()
                .unwrap()
                .status()
                .is_loading()
    });
    assert_eq!(
        sink.timelines(),
        1,
        "cancelled load must not start a late preview"
    );
    app.switch_to_primary_screen(PrimaryScreen::DrumSequencer, None);
    assert!(app.drum_sequencer.screen.cell_on(36, 0));
    assert_eq!(sink.timelines(), 1, "roundtrip does not auto play");
}

#[test]
fn kit_reconfirmation_overlay_quit_and_shutdown_stop_existing_preview() {
    let sink = Arc::new(RecordingSink::default());
    let (_dirs, mut app) = app_with_sink(Arc::clone(&sink));
    *app.patch_load_state.lock().unwrap() = crate::tui::PatchLoadState::Loading;
    // All keys are consumed by the selector, including global shortcuts and q/P.
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Enter));
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Char('t')));
    assert!(!app.can_open_screen_switch_menu());
    for k in [
        preview(),
        key(KeyCode::Char('q')),
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
    ] {
        assert!(!app.dispatch_drum_sequencer_key_event(k));
    }
    assert!(sink.operations().is_empty());
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Esc));
    use cmrt_tui_core::patch_load::{PatchCatalogSnapshot, PatchLoadMeasurement};
    *app.patch_load_state.lock().unwrap() =
        crate::tui::PatchLoadState::Ready(Arc::new(PatchCatalogSnapshot::new(
            vec![("Kit.sfz".to_string(), "kit.sfz".to_string())],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            [(
                "Kit.sfz".to_string(),
                PatchLoadMeasurement {
                    drum_kit: true,
                    drum_kit_notes: Some(vec![36, 38]),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
        )));
    play(&mut app, &sink);
    let stops = sink.stops();
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Char('t')));
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Enter));
    wait_until(|| sink.stops() > stops);
    assert!(
        app.drum_sequencer.screen.cell_on(36, 0),
        "reconfirmation consumes Enter"
    );
    play(&mut app, &sink);
    let stops = sink.stops();
    let timelines = sink.timelines();
    assert!(app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)));
    wait_until(|| sink.stops() > stops);
    app.handle_mml_overlay_key_event(key(KeyCode::Esc));
    assert_eq!(
        sink.timelines(),
        timelines,
        "closing overlay itself does not resume; the frame pump does"
    );
    play(&mut app, &sink);
    let stops = sink.stops();
    assert!(app.dispatch_drum_sequencer_key_event(key(KeyCode::Char('q'))));
    wait_until(|| sink.stops() > stops);
    play(&mut app, &sink);
    let stops = sink.stops();
    drop(app);
    assert!(
        sink.stops() > stops,
        "sender shutdown clears scheduled events"
    );
}

#[test]
fn shift_p_conventions_are_press_only_and_menu_roundtrip_keeps_cells() {
    let sink = Arc::new(RecordingSink::default());
    let (_dirs, mut app) = app_with_sink(Arc::clone(&sink));
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Char(' ')));
    for (code, modifiers, expected) in [
        (KeyCode::Char('P'), KeyModifiers::NONE, true),
        (KeyCode::Char('P'), KeyModifiers::SHIFT, true),
        (KeyCode::Char('p'), KeyModifiers::SHIFT, true),
        (KeyCode::Char('p'), KeyModifiers::NONE, false),
        (KeyCode::Char('P'), KeyModifiers::CONTROL, false),
    ] {
        assert_eq!(is_preview_key(KeyEvent::new(code, modifiers)), expected);
    }
    for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
        let mut k = preview();
        k.kind = kind;
        app.dispatch_drum_sequencer_key_event(k);
    }
    assert!(sink.operations().is_empty());
    assert!(!app.uses_textarea_cursor() && !app.uses_mouse_capture());
    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);
    assert!(
        app.try_open_screen_switch_menu(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL))
    );
    let target = app
        .handle_screen_switch_menu_key(key(KeyCode::Char('s')))
        .unwrap();
    assert_eq!(target, PrimaryScreen::DrumSequencer);
    app.switch_to_primary_screen(target, None);
    assert!(app.drum_sequencer.screen.cell_on(36, 0));
    assert_eq!(sink.timelines(), 0);
    let text = crate::tui::ui::tests::render_lines(&mut app, 120, 24).join("\n");
    let compact: String = text.chars().filter(|ch| !ch.is_whitespace()).collect();
    assert!(text.contains("Drum Sequencer") && compact.contains("Shift+P:再生/停止"));
}

#[test]
fn failed_patch_preparation_reports_reason_and_keeps_input() {
    let sink = Arc::new(RecordingSink::failing_prepare("fixture kit load failure"));
    let (_dirs, mut app) = app_with_sink(Arc::clone(&sink));
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Char(' ')));
    app.dispatch_drum_sequencer_key_event(preview());
    wait_until(|| {
        app.mml_overlay_sender
            .as_ref()
            .unwrap()
            .status()
            .prepare_error()
            .is_some()
    });
    let text = crate::tui::ui::tests::render_lines(&mut app, 120, 24).join("\n");
    assert!(text.contains("fixture kit load failure"));
    assert!(app.drum_sequencer.screen.cell_on(36, 0));
    assert_eq!(sink.timelines(), 0);
    assert!(app.sound_startup_wait.is_none());
    assert!(
        app.drum_sequencer.loop_hits.is_none(),
        "a failed loop is not running, so the next Shift+P starts again"
    );
    app.dispatch_drum_sequencer_key_event(preview());
    wait_until(|| sink.prepared().len() == 2);
}

#[test]
fn help_captures_global_and_matrix_keys_without_stopping_the_preview() {
    let sink = Arc::new(RecordingSink::default());
    let (_dirs, mut app) = app_with_sink(Arc::clone(&sink));
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Char(' ')));
    play(&mut app, &sink);
    let (stops, timelines) = (sink.stops(), sink.timelines());

    for close in [KeyCode::Esc, KeyCode::Char('?')] {
        assert!(!app.dispatch_drum_sequencer_key_event(key(KeyCode::Char('?'))));
        assert!(app.drum_sequencer.captures_keys());
        assert!(!app.can_open_screen_switch_menu());
        assert!(!app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)));
        assert!(!app.mml_overlay.is_open());
        for code in [
            KeyCode::Char('t'),
            KeyCode::Char('P'),
            KeyCode::Char('l'),
            KeyCode::Enter,
            KeyCode::Char('q'),
        ] {
            assert!(
                !app.dispatch_drum_sequencer_key_event(key(code)),
                "{code:?}"
            );
        }
        assert!(!app.drum_sequencer.selector_open(), "t must not open kits");
        assert!(!app.dispatch_drum_sequencer_key_event(key(close)));
        assert!(!app.drum_sequencer.captures_keys(), "{close:?}");
        assert!(app.can_open_screen_switch_menu());
    }
    let screen = &app.drum_sequencer.screen;
    assert_eq!((screen.cursor_note(), screen.cursor_step()), (Some(36), 0));
    assert!(screen.cell_on(36, 0));
    assert_eq!(
        sink.stops(),
        stops,
        "help must not stop the running preview"
    );
    assert_eq!(
        sink.timelines(),
        timelines,
        "Shift+P behind help must not play"
    );
    assert!(app.drum_sequencer.preview_command.is_some());
}

#[test]
fn audition_program_is_ascending_quarter_second_note_ons_of_existing_notes_only() {
    let program = kit_audition_program(&[70, 36, 42, 42]);
    assert!(!program.repeat);
    let ons: Vec<_> = program
        .events()
        .iter()
        .filter(|event| event.message[0] == 0x90)
        .map(|event| (event.seconds, event.message[1]))
        .collect();
    assert_eq!(ons, [(0.0, 36), (0.25, 42), (0.5, 70)]);
}

mod auto_play;
mod step_loop;
