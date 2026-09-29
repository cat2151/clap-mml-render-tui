//! Guitar Articulation 画面の操作（MML の確定・`b` / `space`・ルールの toggle）が、
//! どの版を・どの音色で `play_line` へ渡すか。
//!
//! 音そのものは機械で判定できないので、記録 sender が受けた timeline の中身を読む。

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::*;
use crate::screen_switch::PrimaryScreen;
use crate::tui::guitar_articulation::{convert, Rule, RuleTable, DEFAULT_MML, PATCH};
use cmrt_mml_overlay::{LivePatch, MmlOverlaySender, RecordingSink};

const MML: &str = "o3 l8 e f+ g";

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() {
        assert!(Instant::now() < deadline, "待ちきれなかった: {what}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn attach_recording_sender(app: &mut TuiApp<'_>) -> Arc<RecordingSink> {
    let sink = Arc::new(RecordingSink::default());
    app.mml_overlay_sender = Some(MmlOverlaySender::with_recording_sink(
        Arc::clone(&sink),
        48_000.0,
    ));
    sink
}

/// MML 欄の中身を `MML` に打ち替えて確定する。入った直後は欄が開いていて既定の MML が入っている。
fn type_mml(app: &mut TuiApp<'_>) {
    if !app.guitar_articulation.input_open() {
        app.handle_guitar_articulation_key_event(plain(KeyCode::Char('i')));
    }
    for code in [KeyCode::Char('a'), KeyCode::Char('k')] {
        app.handle_guitar_articulation_key_event(KeyEvent::new(code, KeyModifiers::CONTROL));
    }
    for ch in MML.chars() {
        app.handle_guitar_articulation_key_event(plain(KeyCode::Char(ch)));
    }
    app.handle_guitar_articulation_key_event(plain(KeyCode::Enter));
}

/// MML を確定した Guitar Articulation 画面。確定でも鳴るので、記録 sender は確定の後で付け、
/// 各テストが見る記録を「その後の操作」の分だけにする。
fn app_with_mml<'a>() -> (TuiApp<'a>, Arc<RecordingSink>) {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);
    type_mml(&mut app);
    let sink = attach_recording_sender(&mut app);
    (app, sink)
}

fn flat_events() -> Vec<cmrt_chord::TimedMidiEvent> {
    cmrt_chord::timed_performance(MML).unwrap().events
}

/// 受けた順の timeline の message のうち、`from` 番目以降。
fn sent_messages(sink: &RecordingSink, from: usize) -> Vec<[u8; 3]> {
    sink.timeline_events()[from..]
        .iter()
        .map(|event| event.message)
        .collect()
}

fn messages(events: &[cmrt_chord::TimedMidiEvent]) -> Vec<[u8; 3]> {
    events.iter().map(|event| event.message).collect()
}

#[test]
fn the_screen_switch_menu_opens_it_with_e() {
    let mut app = TuiApp::new_for_test(test_config());
    app.screen_switch_menu.open();

    let target = app.handle_screen_switch_menu_key(plain(KeyCode::Char('e')));

    assert_eq!(target, Some(PrimaryScreen::GuitarArticulation));
    app.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);
    assert_eq!(app.active_screen, PrimaryScreen::GuitarArticulation);
}

fn default_converted() -> Vec<cmrt_chord::TimedMidiEvent> {
    let flat = cmrt_chord::timed_performance(DEFAULT_MML).unwrap().events;
    convert(&flat, &RuleTable::default())
}

/// 初回は METAL-GTX を読み始め、MML 欄を開き、既定の MML のArticulatedを鳴らす。
#[test]
fn entering_the_screen_first_time_opens_the_input_and_plays_the_default_mml() {
    let mut app = TuiApp::new_for_test(test_config());
    let sink = attach_recording_sender(&mut app);
    let converted = default_converted();

    app.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);

    assert!(app.guitar_articulation.input_open());
    assert_eq!(app.guitar_articulation.mml(), DEFAULT_MML);
    wait_until("既定の MML のArticulated", || {
        sink.timeline_events().len() >= converted.len()
    });
    assert_eq!(sink.prepared(), vec![LivePatch::new(Some(PATCH))]);
    assert_eq!(sent_messages(&sink, 0), messages(&converted));
    assert_eq!(sink.timelines(), 1);
}

/// 前回この画面で終えて起動したときも、メニューから入ったときと同じく既定の MML を入れ、
/// METAL-GTX を読み始める。
#[test]
fn starting_the_app_on_the_screen_fills_the_default_mml_and_prepares_the_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    let sink = attach_recording_sender(&mut app);
    app.active_screen = PrimaryScreen::GuitarArticulation;

    app.enter_restored_guitar_articulation();

    assert_eq!(app.guitar_articulation.mml(), DEFAULT_MML);
    wait_until("音色の用意", || !sink.prepared().is_empty());
    assert_eq!(sink.prepared(), vec![LivePatch::new(Some(PATCH))]);
}

/// MML が入っていれば、入り直しても鳴らさない（MML 欄は開く）。
#[test]
fn entering_again_with_an_mml_opens_the_input_without_playing() {
    let (mut app, sink) = app_with_mml();
    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);

    app.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);

    assert!(app.guitar_articulation.input_open());
    assert_eq!(app.guitar_articulation.mml(), MML);
    wait_until("音色の用意", || sink.prepared().len() == 1);
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(sink.timelines(), 0);
}

#[test]
fn committing_the_mml_plays_the_converted_take() {
    let mut app = TuiApp::new_for_test(test_config());
    let sink = attach_recording_sender(&mut app);
    app.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);
    let first = default_converted().len();
    wait_until("既定の MML", || sink.timeline_events().len() >= first);
    let converted = convert(&flat_events(), &RuleTable::default());

    type_mml(&mut app);
    wait_until("Articulatedが積まれる", || {
        sink.timeline_events().len() >= first + converted.len()
    });

    assert_eq!(sent_messages(&sink, first), messages(&converted));
    assert_eq!(sink.timelines(), 2);
}

/// MML 欄が開いたまま入るので、入力中でも `Ctrl+G` で画面を出られる。help 表示中は出ない。
#[test]
fn ctrl_g_opens_the_screen_switch_menu_while_typing_but_not_over_the_help() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);
    let ctrl_g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
    assert!(app.guitar_articulation.input_open());

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('?')));
    assert!(!app.try_open_screen_switch_menu(ctrl_g));

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('?')));
    assert!(app.try_open_screen_switch_menu(ctrl_g));
}

#[test]
fn h_and_l_alone_do_not_play() {
    let (mut app, sink) = app_with_mml();

    for code in ['l', 'l', 'h'] {
        app.handle_guitar_articulation_key_event(plain(KeyCode::Char(code)));
    }

    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(sink.timelines(), 0);
    assert_eq!(app.guitar_articulation.cursor(), 1);
}

#[test]
fn l_then_a_plays_the_take_with_the_hammer_on_and_a_again_turns_it_off() {
    let (mut app, sink) = app_with_mml();
    let flat = flat_events();
    let without = convert(&flat, &RuleTable::default());
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::HammerPull);
    let with = convert(&flat, &rules);
    // 対照: ルールでArticulatedが変わる。
    assert_ne!(messages(&with), messages(&without));

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('l')));
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('a')));
    wait_until("ON のArticulated", || {
        sink.timeline_events().len() >= with.len()
    });
    assert_eq!(sent_messages(&sink, 0), messages(&with));
    assert!(sent_messages(&sink, 0).contains(&[0x90, 26, 127]));

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('a')));
    wait_until("OFF のArticulated", || {
        sink.timeline_events().len() >= with.len() + without.len()
    });
    assert_eq!(sent_messages(&sink, with.len()), messages(&without));
    assert_eq!(sink.timelines(), 2);
}

#[test]
fn b_plays_the_flat_events_as_they_are() {
    let (mut app, sink) = app_with_mml();
    let flat = flat_events();

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('b')));
    wait_until("rawが積まれる", || {
        sink.timeline_events().len() >= flat.len()
    });

    assert_eq!(sent_messages(&sink, 0), messages(&flat));
    assert_eq!(sink.prepared(), vec![LivePatch::new(Some(PATCH))]);
}

#[test]
fn space_plays_the_converted_events_with_the_keyswitches() {
    let (mut app, sink) = app_with_mml();
    let converted = convert(&flat_events(), &RuleTable::default());
    // 対照: Articulatedはrawと違う（先頭の KS 17 がある）。
    assert_ne!(messages(&converted), messages(&flat_events()));

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char(' ')));
    wait_until("Articulatedが積まれる", || {
        sink.timeline_events().len() >= converted.len()
    });

    assert_eq!(sent_messages(&sink, 0), messages(&converted));
    assert_eq!(sent_messages(&sink, 0)[0], [0x90, 17, 127]);
}

#[test]
fn b_then_space_sends_each_take_in_turn() {
    let (mut app, sink) = app_with_mml();
    let flat = flat_events();
    let converted = convert(&flat, &RuleTable::default());

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('b')));
    wait_until("raw", || sink.timeline_events().len() >= flat.len());
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char(' ')));
    wait_until("Articulated", || {
        sink.timeline_events().len() >= flat.len() + converted.len()
    });

    assert_eq!(sent_messages(&sink, flat.len()), messages(&converted));
    assert_eq!(sink.timelines(), 2);
}

#[test]
fn leaving_the_screen_stops_the_take() {
    let (mut app, sink) = app_with_mml();
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char(' ')));
    wait_until("Articulatedが積まれる", || sink.timelines() == 1);
    let stops = sink.stops();

    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);

    wait_until("止まる", || sink.stops() > stops);
}

#[test]
fn the_app_draws_the_screen_with_both_takes() {
    let (mut app, _sink) = app_with_mml();

    let screen = crate::tui::ui::tests::render_lines(&mut app, 100, 30)
        .join("\n")
        .replace(' ', "");

    assert!(screen.contains("GuitarArticulation"), "{screen}");
    assert!(screen.contains("KSSus_Down"), "{screen}");
    assert!(screen.contains("F#2"), "{screen}");
}
