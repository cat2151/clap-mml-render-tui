//! Guitar Articulation 画面は Lite で鳴らし始め、裏で Full を先読みし、読み終えたら
//! 次の再生から Full で鳴らす。
//!
//! 先読みの完了は記録 sender の `finish_preload` で進める。どの bank で鳴ったかは
//! timeline の instance で見る（Lite は instance 0、先読みした Full は instance 1）。

use cmrt_mml_overlay::{MmlOverlayPreload, MmlOverlaySender, RecordingSink};

use super::guitar_articulation::{
    app_with_mml_and_effects, attach_recording_sender, first_candidate_chain, plain, wait_until,
};
use super::*;
use crate::screen_switch::PrimaryScreen;
use crate::tui::guitar_articulation::{Instrument, StartupInstrument, Take, FULL_PATCH, PATCH};
use cmrt_mml_overlay::LivePatch;

fn last_timeline_instance(sink: &RecordingSink) -> u8 {
    sink.timeline_events()
        .last()
        .expect("timeline が張られている")
        .instance_id
}

fn preload_state(app: &TuiApp<'_>) -> Option<MmlOverlayPreload> {
    app.mml_overlay_sender
        .as_ref()
        .and_then(|sender| MmlOverlaySender::status(sender).preload().cloned())
}

/// `space`（Articulated）を押し、`count` 本目の timeline にその列が積まれるまで待つ。
fn play_converted(app: &mut TuiApp<'_>, sink: &RecordingSink, count: usize) {
    let sent = sink.timeline_events().len();
    let expected = app.guitar_articulation.events(Take::Converted).len();
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char(' ')));
    wait_until("Articulated の timeline", || {
        sink.timelines() == count && sink.timeline_events().len() >= sent + expected
    });
}

/// 先読みを完了させ、sender がそれを見せるまで待つ。
fn finish_full_preload(app: &TuiApp<'_>, sink: &RecordingSink, full: &LivePatch) {
    sink.finish_preload();
    wait_until("Full の先読みの完了", || {
        preload_state(app) == Some(MmlOverlayPreload::Ready(full.clone()))
    });
}

#[test]
fn entering_prepares_lite_then_preloads_full_and_plays_full_only_after_it_finished() {
    let mut app = TuiApp::new_for_test(test_config());
    let sink = attach_recording_sender(&mut app);
    let full = LivePatch::new(Some(FULL_PATCH));

    // 入ると既定の MML を Lite で鳴らし、Full を裏へ頼む。
    app.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);
    wait_until("Full の先読みの受付", || {
        sink.timelines() == 1 && !sink.timeline_events().is_empty() && !sink.preloads().is_empty()
    });
    assert_eq!(sink.prepared(), vec![LivePatch::new(Some(PATCH))]);
    assert_eq!(sink.preloads(), vec![full.clone()]);
    assert_eq!(last_timeline_instance(&sink), 0);
    wait_until("先読み中の表示", || {
        app.sync_guitar_articulation_instrument("") == Instrument::LiteLoadingFull
    });
    assert!(!app
        .mml_overlay_sender
        .as_ref()
        .unwrap()
        .status()
        .is_loading());
    // 入ると MML 欄が開いているので、matrix の操作へ戻る。
    app.handle_guitar_articulation_key_event(plain(KeyCode::Esc));

    // 完了前の再生は Lite のまま、待たずに鳴る。先読みを頼み直しても読み直さない。
    play_converted(&mut app, &sink, 2);
    assert_eq!(last_timeline_instance(&sink), 0);
    assert_eq!(sink.prepared(), vec![LivePatch::new(Some(PATCH))]);
    assert_eq!(sink.preloads(), vec![full.clone()]);

    // 完了後の再生は Full の bank で、読み込み無しに鳴る。
    finish_full_preload(&app, &sink, &full);
    play_converted(&mut app, &sink, 3);
    assert_eq!(last_timeline_instance(&sink), 1);
    assert_eq!(sink.prepared(), vec![LivePatch::new(Some(PATCH))]);
    assert_eq!(app.guitar_articulation.instrument(), Instrument::Full);
    assert_eq!(sink.preloads(), vec![full]);
}

#[test]
fn after_full_is_ready_changing_the_effect_chain_keeps_full() {
    let (mut app, sink) = app_with_mml_and_effects();
    let full = LivePatch::new(Some(FULL_PATCH));
    play_converted(&mut app, &sink, 1);
    wait_until("Full の先読みの受付", || !sink.preloads().is_empty());
    finish_full_preload(&app, &sink, &full);
    play_converted(&mut app, &sink, 2);
    assert_eq!(app.guitar_articulation.instrument(), Instrument::Full);

    for code in [
        KeyCode::Char('x'),
        KeyCode::Char('a'),
        KeyCode::Enter,
        KeyCode::Enter,
    ] {
        app.handle_guitar_articulation_key_event(plain(code));
    }
    let played = sink.timelines();
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('b')));
    wait_until("chain を替えた後の raw", || sink.timelines() > played);

    assert_eq!(app.guitar_articulation.instrument(), Instrument::Full);
    // 最初の Lite の準備より後は、試聴も確定後の演奏も Full だけを読む。
    let after_full = sink.prepared()[1..].to_vec();
    assert_eq!(
        after_full.last(),
        Some(&LivePatch::with_effect_chain(
            Some(FULL_PATCH),
            &first_candidate_chain()
        ))
    );
    assert!(
        after_full
            .iter()
            .all(|patch| patch.patch() == Some(FULL_PATCH)),
        "Lite へ戻った: {after_full:?}"
    );
    assert_eq!(sink.preloads(), vec![full], "Full になった後は先読みしない");
}

/// 先読みの最中に chain を替えると、server は先読みの完了まで待たせる。`w` はその間効かず、
/// 何も送らない（Lite の読み直しも Full の先読みの頼み直しもしない）。読み終えたら効く。
#[test]
fn w_does_nothing_while_full_loads_and_works_after_it_finished() {
    let mut app = TuiApp::new_for_test(test_config());
    let sink = attach_recording_sender(&mut app);
    let full = LivePatch::new(Some(FULL_PATCH));
    app.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);
    wait_until("Lite の timeline", || sink.timelines() == 1);
    wait_until("先読み中の表示", || {
        app.sync_guitar_articulation_instrument("") == Instrument::LiteLoadingFull
    });
    app.handle_guitar_articulation_key_event(plain(KeyCode::Esc));

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('w')));

    assert!(!app.guitar_articulation.effect_dry());
    assert!(app.guitar_articulation.error.is_some());
    assert_eq!(sink.timelines(), 1);
    assert_eq!(sink.preloads(), vec![full.clone()]);

    finish_full_preload(&app, &sink, &full);
    app.sync_guitar_articulation_instrument("");
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('w')));
    assert!(app.guitar_articulation.effect_dry());
    wait_until("dry の timeline", || sink.timelines() == 2);
}

#[test]
fn startup_full_prepares_full_on_entering_and_never_preloads() {
    let mut app = TuiApp::new_for_test(test_config());
    let sink = attach_recording_sender(&mut app);
    app.guitar_articulation = std::mem::take(&mut app.guitar_articulation)
        .with_startup_instrument(StartupInstrument::Full);

    app.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);
    wait_until("Full の timeline", || {
        sink.timelines() == 1 && !sink.timeline_events().is_empty()
    });
    assert_eq!(sink.prepared(), vec![LivePatch::new(Some(FULL_PATCH))]);
    assert!(sink.preloads().is_empty());
    assert_eq!(app.guitar_articulation.instrument(), Instrument::Full);

    app.handle_guitar_articulation_key_event(plain(KeyCode::Esc));
    play_converted(&mut app, &sink, 2);
    assert_eq!(sink.prepared(), vec![LivePatch::new(Some(FULL_PATCH))]);
    assert!(sink.preloads().is_empty());
}
