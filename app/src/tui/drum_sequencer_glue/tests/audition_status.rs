use super::audition::{app_with_sink, wait_until};
use super::*;
use cmrt_mml_overlay::{LivePatch, RecordingSink};
use cmrt_tui_core::patch_load::HEAVY_OFFLINE_LOAD_BYTES;

/// A と B は軽い kit、C は重い kit。
fn kits_with_heavy() -> PatchLoadState {
    let kits = [
        ("A Kit.sfz", None),
        ("B 909.sfz", None),
        ("C Heavy.sfz", Some(HEAVY_OFFLINE_LOAD_BYTES * 2)),
    ];
    let measurements = kits
        .iter()
        .map(|(name, bytes)| {
            (
                name.to_string(),
                PatchLoadMeasurement {
                    drum_kit: true,
                    drum_kit_notes: Some(vec![36, 38]),
                    sfz_sample_bytes: *bytes,
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

fn lines(app: &mut TuiApp<'static>) -> Vec<String> {
    crate::tui::ui::tests::render_lines(app, 120, 30)
}

/// 全角文字の後ろに TestBackend が入れる空白ごと、空白を除いた画面の文字。
fn compact(lines: &[String]) -> String {
    lines
        .concat()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

/// 一覧の `patch` の行。状態行は `[kit]` を含まないので取り違えない。
fn row<'a>(lines: &'a [String], patch: &str) -> &'a str {
    lines
        .iter()
        .find(|line| line.contains(&format!("{patch} [kit]")))
        .unwrap_or_else(|| panic!("row for {patch}: {lines:#?}"))
}

#[test]
fn heavy_kit_is_not_auditioned_by_moving_onto_it_only_by_space() {
    let sink = Arc::new(RecordingSink::default());
    let mut app = app_with_sink(kits_with_heavy(), &sink);
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
    wait_until(|| sink.timelines() == 1);

    let stops = sink.stops();
    app.handle_drum_sequencer_key_event(press(KeyCode::End));
    assert_eq!(selected(&app).selected(), Some("C Heavy.sfz"));
    wait_until(|| sink.stops() > stops);
    assert!(app.drum_sequencer.preview_command.is_none());
    let text = compact(&lines(&mut app));
    assert!(text.contains("重いkit(128MB)"), "{text}");
    assert!(
        !sink
            .prepared()
            .contains(&LivePatch::new(Some("C Heavy.sfz"))),
        "moving onto a heavy kit must not load it"
    );

    app.handle_drum_sequencer_key_event(press(KeyCode::Char(' ')));
    wait_until(|| sink.timelines() == 2);
    assert_eq!(
        sink.prepared().last(),
        Some(&LivePatch::new(Some("C Heavy.sfz")))
    );
}

#[test]
fn status_line_is_drawn_inside_the_selector_and_sounding_row_is_marked() {
    let sink = Arc::new(RecordingSink::default());
    let mut app = app_with_sink(kits_with_heavy(), &sink);
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
    wait_until(|| {
        app.mml_overlay_sender
            .as_ref()
            .unwrap()
            .status()
            .line_playback()
            .is_some()
    });
    let lines = lines(&mut app);
    let status_row = lines
        .iter()
        .position(|line| compact(std::slice::from_ref(line)).contains("試聴中"))
        .unwrap_or_else(|| panic!("{lines:#?}"));
    let panes_bottom = lines
        .iter()
        .position(|line| line.contains("┘└"))
        .expect("pane bottom border");
    assert_eq!(
        status_row,
        panes_bottom + 1,
        "status row sits inside the selector, just under its panes: {lines:#?}"
    );
    assert!(compact(&lines).contains("Drumkit選択中"));
    assert!(row(&lines, "A Kit.sfz").contains('♪'));
    assert!(!row(&lines, "B 909.sfz").contains('♪'));
}

#[test]
fn load_of_a_previous_candidate_is_shown_as_waiting_and_marks_its_row() {
    let sink = Arc::new(RecordingSink::default());
    let mut app = app_with_sink(kits_with_heavy(), &sink);
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
    wait_until(|| sink.timelines() == 1);

    // 裏の先読みが終わるまで、次の候補の読み込みは終わらない。
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
    let own = lines(&mut app);
    assert!(compact(&own).contains("読込中"), "{own:#?}");
    assert!(row(&own, "B 909.sfz").contains('…'));

    // 重い kit へ移ると自分の試聴は止めるが、B の読み込みは中断できない。
    app.handle_drum_sequencer_key_event(press(KeyCode::Down));
    let other = lines(&mut app);
    assert!(compact(&other).contains("読込待ち"), "{other:#?}");
    assert!(row(&other, "B 909.sfz").contains('…'));
    assert!(!row(&other, "C Heavy.sfz").contains('…'));
    sink.finish_preload();
}
