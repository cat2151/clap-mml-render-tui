use super::*;
use cmrt_mml_overlay::{LivePatch, MmlOverlaySender, RecordingSink, SinkOperation};
use std::time::{Duration, Instant};

pub(super) fn kits(kits: &[(&str, Option<Vec<u8>>)]) -> PatchLoadState {
    let measurements = kits
        .iter()
        .map(|(name, notes)| {
            (
                name.to_string(),
                PatchLoadMeasurement {
                    drum_kit: true,
                    drum_kit_notes: notes.clone(),
                    ..Default::default()
                },
            )
        })
        .collect();
    let pairs = kits
        .iter()
        .map(|(name, _)| (name.to_string(), name.to_lowercase()))
        .collect();
    PatchLoadState::Ready(Arc::new(PatchCatalogSnapshot::new(
        pairs,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        measurements,
    )))
}

pub(super) fn wait_until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !ready() {
        assert!(Instant::now() < deadline, "sender condition timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
}

pub(super) fn app_with_sink(state: PatchLoadState, sink: &Arc<RecordingSink>) -> TuiApp<'static> {
    let mut app = app_with(state);
    app.mml_overlay_sender = Some(MmlOverlaySender::with_recording_sink(
        Arc::clone(sink),
        48_000.0,
    ));
    app.switch_to_primary_screen(crate::screen_switch::PrimaryScreen::DrumSequencer, None);
    app
}

/// `from_event` 以降の timeline の note-on を、その先頭からの秒と note の組で返す。
fn note_ons_from(sink: &RecordingSink, from_event: usize) -> Vec<(f64, u8)> {
    let events = sink.timeline_events()[from_event..].to_vec();
    let start = events.first().map_or(0.0, |event| event.timeline_seconds);
    events
        .iter()
        .filter(|event| event.message[0] == 0x90)
        .map(|event| {
            (
                ((event.timeline_seconds - start) * 1000.0).round() / 1000.0,
                event.message[1],
            )
        })
        .collect()
}

fn screen_text(app: &mut TuiApp<'static>) -> String {
    crate::tui::ui::tests::render_lines(app, 120, 30)
        .concat()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

#[test]
fn kit_list_auditions_each_candidate_once_in_ascending_notes_and_space_replays() {
    let sink = Arc::new(RecordingSink::default());
    let mut app = app_with_sink(
        kits(&[
            ("A Kit.sfz", Some(vec![36, 42, 70])),
            ("B 909.sfz", Some(vec![38, 42])),
            ("C Unknown.sfz", None),
            ("D Empty.sfz", Some(Vec::new())),
        ]),
        &sink,
    );
    // matrix の ON セル（kit に無い 37）は試聴に混ざらない。
    app.drum_sequencer.screen.set_kit(
        "B 909.sfz".to_string(),
        Some(vec![37, 38]),
        Vec::new(),
        Vec::new(),
    );
    app.handle_drum_sequencer_key_event(press(KeyCode::Enter));

    app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
    assert_eq!(selected(&app).selected(), Some("B 909.sfz"));
    wait_until(|| sink.timelines() == 1);
    assert_eq!(note_ons_from(&sink, 0), [(0.0, 38), (0.25, 42)]);

    let from = sink.timeline_events().len();
    app.handle_drum_sequencer_key_event(press(KeyCode::Home));
    wait_until(|| sink.timelines() == 2);
    assert_eq!(
        note_ons_from(&sink, from),
        [(0.0, 36), (0.25, 42), (0.5, 70)]
    );
    assert_eq!(
        sink.prepared().last(),
        Some(&LivePatch::new(Some("A Kit.sfz")))
    );
    assert_eq!(
        app.drum_sequencer.screen.kit_name(),
        Some("B 909.sfz"),
        "audition does not change the editing kit"
    );

    let from = sink.timeline_events().len();
    let stops = sink.stops();
    app.handle_drum_sequencer_key_event(press(KeyCode::Char(' ')));
    wait_until(|| sink.timelines() == 3);
    assert!(sink.stops() > stops, "replay restarts from the top");
    assert_eq!(
        note_ons_from(&sink, from),
        [(0.0, 36), (0.25, 42), (0.5, 70)]
    );

    let stops = sink.stops();
    // 鳴る候補（B）を経由すると、その試聴が正しく出てしまうので End / Up で直接移る。
    for (code, patch, reason) in [
        (KeyCode::End, "D Empty.sfz", "割当noteなし"),
        (KeyCode::Up, "C Unknown.sfz", "割当note不明"),
    ] {
        app.handle_drum_sequencer_key_event(press(code));
        assert_eq!(selected(&app).selected(), Some(patch));
        assert!(app.drum_sequencer.preview_command.is_none());
        wait_until(|| sink.stops() > stops);
        app.handle_drum_sequencer_key_event(press(KeyCode::Char(' ')));
        let text = screen_text(&mut app);
        assert!(text.contains(reason), "{patch}: {text}");
    }
    assert_eq!(sink.timelines(), 3, "unknown / empty kits never play");

    // 検索入力中の Space は検索欄へ渡し、試聴しない。
    app.handle_drum_sequencer_key_event(press(KeyCode::Home));
    wait_until(|| sink.timelines() == 4);
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('/')));
    app.handle_drum_sequencer_key_event(press(KeyCode::Char(' ')));
    assert!(selected(&app).filter_editing());
    app.handle_drum_sequencer_key_event(press(KeyCode::Esc));
    assert!(app.drum_sequencer.selector_open());

    let stops = sink.stops();
    app.handle_drum_sequencer_key_event(press(KeyCode::Esc));
    wait_until(|| sink.stops() > stops);
    assert!(!app.drum_sequencer.selector_open());
    assert_eq!(app.drum_sequencer.screen.kit_name(), Some("B 909.sfz"));
    assert!(app.drum_sequencer.screen.cell_on(37, 0));
    assert_eq!(sink.timelines(), 4);
    assert_eq!(sink.operations().last(), Some(&SinkOperation::Stop));
}

#[test]
fn search_to_no_candidate_and_cancel_while_loading_never_start_a_late_audition() {
    let sink = Arc::new(RecordingSink::default());
    let mut app = app_with_sink(
        kits(&[("A Kit.sfz", Some(vec![36])), ("B 909.sfz", Some(vec![38]))]),
        &sink,
    );
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
    wait_until(|| sink.timelines() == 1);

    // 絞り込みで候補が消えたら止め、同じ候補へ戻ったら鳴らし直す。
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('/')));
    let stops = sink.stops();
    for ch in "zzz".chars() {
        app.handle_drum_sequencer_key_event(press(KeyCode::Char(ch)));
    }
    assert_eq!(selected(&app).filtered_len(), 0);
    wait_until(|| sink.stops() > stops);
    app.handle_drum_sequencer_key_event(press(KeyCode::Esc));
    wait_until(|| sink.timelines() == 2);

    let sender = app.mml_overlay_sender.as_ref().unwrap();
    sender.preload(LivePatch::new(Some("Background.sfz")));
    wait_until(|| !sink.preloads().is_empty());
    app.handle_drum_sequencer_key_event(press(KeyCode::Down));
    wait_until(|| {
        app.mml_overlay_sender
            .as_ref()
            .unwrap()
            .status()
            .is_loading()
    });
    let text = screen_text(&mut app);
    assert!(text.contains("読込中"), "{text}");
    app.handle_drum_sequencer_key_event(press(KeyCode::Esc));
    sink.finish_preload();
    wait_until(|| {
        let status = app.mml_overlay_sender.as_ref().unwrap().status();
        !status.is_loading() && status.line_playback().is_none()
    });
    assert_eq!(
        sink.timelines(),
        2,
        "cancelled candidate must not start after its load"
    );
    assert!(app.drum_sequencer.screen.kit_name().is_none());
}

#[test]
fn reservation_opened_after_catalog_load_auditions_the_first_candidate() {
    let sink = Arc::new(RecordingSink::default());
    let mut app = app_with_sink(PatchLoadState::Loading, &sink);
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
    *app.patch_load_state.lock().unwrap() = kits(&[("A Kit.sfz", Some(vec![40, 36]))]);
    app.sync_drum_sequencer_catalog();
    wait_until(|| sink.timelines() == 1);
    assert_eq!(note_ons_from(&sink, 0), [(0.0, 36), (0.25, 40)]);
    app.sync_drum_sequencer_catalog();
    assert_eq!(app.drum_sequencer.take_kit_audition(), None);
    assert_eq!(sink.timelines(), 1, "per-frame sync does not replay");
}
