use std::collections::HashSet;
use std::time::{Duration, Instant};

use cmrt_tui_core::bpm::{BpmMode, BpmRange};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{GridSequencerScreen, LOOKAHEAD, STEP_INTERVAL};

/// テンポ乗り換えを何周ぶんも見るための擬似フレーム間隔と回数。
/// 30秒ぶん回すので、BPM80（1周3秒）でも10周は跨ぐ。
const FRAME: Duration = Duration::from_millis(50);
const FRAMES: u32 = 600;

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

/// Ctrl+B を開いて `input` を打ち、Enter で確定する。
fn enter_bpm(screen: &mut GridSequencerScreen, now: Instant, input: &str) {
    let ctx = crate::tests::ready_ctx(&[]);
    screen.handle_key(key(KeyCode::Char('b'), KeyModifiers::CONTROL), now, &ctx);
    for character in input.chars() {
        screen.handle_key(key(KeyCode::Char(character), KeyModifiers::NONE), now, &ctx);
    }
    screen.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), now, &ctx);
}

/// `pump_step` のうちテンポ乗り換えに関わるところだけを回す。
///
/// 接続が Ready かどうかに左右されず、周をまたぐところだけを見たいので
/// `poll_steps` を直接叩く。返すのは組み立てられた絶対 musical time。
fn pump_tempo(screen: &mut GridSequencerScreen, now: Instant) -> Vec<f64> {
    screen.arm_next_cycle_bpm();
    let scheduled = screen.state.poll_steps(now, LOOKAHEAD);
    screen.absorb_applied_cycle_bpm();
    scheduled
        .iter()
        .map(|message| message.timeline_seconds)
        .collect()
}

#[test]
fn ctrl_b_opens_input_and_enter_applies_manual_bpm() {
    let mut screen = GridSequencerScreen::new(None);
    let now = Instant::now();
    enter_bpm(&mut screen, now, "128.125");

    assert_eq!(screen.bpm_mode(), BpmMode::Manual(128.125));
    assert_eq!(screen.bpm(), 128.125);
    assert!(screen.state.is_running());
}

#[test]
fn a_returns_to_auto_and_plain_b_keeps_its_existing_binding() {
    let mut screen = GridSequencerScreen::new(None);
    screen.bpm_mode = BpmMode::Manual(90.0);
    let now = Instant::now();
    let ctx = crate::tests::ready_ctx(&[]);
    screen.handle_key(key(KeyCode::Char('b'), KeyModifiers::NONE), now, &ctx);
    assert!(screen.single_buffering());
    assert!(screen.bpm_input.is_none());

    screen.handle_key(key(KeyCode::Char('b'), KeyModifiers::CONTROL), now, &ctx);
    screen.handle_key(key(KeyCode::Char('a'), KeyModifiers::NONE), now, &ctx);
    assert_eq!(screen.bpm_mode(), BpmMode::Auto(crate::BPM));
    assert_eq!(screen.bpm(), crate::BPM);
}

#[test]
fn a_hyphen_pair_sets_the_range_and_draws_inside_it() {
    let mut screen = GridSequencerScreen::new(None);
    let now = Instant::now();
    enter_bpm(&mut screen, now, "80-160");

    assert_eq!(screen.bpm_range(), BpmRange::new(80.0, 160.0).unwrap());
    assert!(screen.bpm_mode().auto_target().is_some());
    assert!((80.0..=160.0).contains(&screen.bpm()), "{}", screen.bpm());
    assert!(screen.state.is_running());

    // 範囲を保ったまま A で引き直せる。
    let ctx = crate::tests::ready_ctx(&[]);
    screen.handle_key(key(KeyCode::Char('b'), KeyModifiers::CONTROL), now, &ctx);
    screen.handle_key(key(KeyCode::Char('a'), KeyModifiers::NONE), now, &ctx);
    assert_eq!(screen.bpm_range(), BpmRange::new(80.0, 160.0).unwrap());
    assert!((80.0..=160.0).contains(&screen.bpm()), "{}", screen.bpm());
}

#[test]
fn the_automatic_bpm_is_redrawn_every_time_the_grid_wraps() {
    let mut screen = GridSequencerScreen::new(None);
    let now = Instant::now();
    enter_bpm(&mut screen, now, "80-160");

    let mut drawn = HashSet::new();
    drawn.insert(screen.bpm() as i64);
    for frame in 0..FRAMES {
        pump_tempo(&mut screen, now + FRAME * frame);
        assert!((80.0..=160.0).contains(&screen.bpm()), "{}", screen.bpm());
        drawn.insert(screen.bpm() as i64);
    }

    assert!(drawn.len() > 2, "周ごとに引き直されていない: {drawn:?}");
    // 乗り換えでクロックを畳まないこと（演奏は止めずに続く）。
    assert!(screen.state.is_running());
}

/// BPM を1周ごとの random から外したら、範囲を指定していてもテンポは動かない。
#[test]
fn dropping_the_bpm_item_freezes_the_tempo_even_with_a_range() {
    let mut screen = GridSequencerScreen::new(None);
    let now = Instant::now();
    enter_bpm(&mut screen, now, "80-160");
    screen.set_cycle_random(crate::CycleRandomItem::Bpm, false);
    let frozen = screen.bpm();

    for frame in 0..FRAMES {
        pump_tempo(&mut screen, now + FRAME * frame);
    }

    assert!(!screen.state.has_armed_cycle_bpm(), "次の周へ予約もしない");
    assert_eq!(screen.bpm(), frozen);
}

#[test]
fn the_tempo_only_moves_at_a_wrap_not_in_the_middle_of_one() {
    let mut screen = GridSequencerScreen::new(None);
    let now = Instant::now();
    enter_bpm(&mut screen, now, "80-160");

    // 1周ぶんに満たない間は、何度 pump しても BPM は動かない。
    let before = screen.bpm();
    for frame in 0..4 {
        pump_tempo(&mut screen, now + STEP_INTERVAL / 4 * frame);
    }
    assert_eq!(screen.bpm(), before);
}

#[test]
fn a_wrap_tempo_change_keeps_the_musical_timeline_monotonic() {
    // サーバーは絶対 musical time でイベントを並べるので、テンポを乗り換えたときに
    // 秒数が巻き戻ると鳴らし損ねる。
    let mut screen = GridSequencerScreen::new(None);
    let now = Instant::now();
    enter_bpm(&mut screen, now, "80-160");

    let mut last = f64::NEG_INFINITY;
    let mut count = 0usize;
    for frame in 0..FRAMES {
        for seconds in pump_tempo(&mut screen, now + FRAME * frame) {
            assert!(
                seconds >= last,
                "timeline が巻き戻った: {last} -> {seconds}"
            );
            last = seconds;
            count += 1;
        }
    }
    assert!(count > 0, "1つも組み立てていない");
}

/// 周ごとにテンポ変化点がちょうど1回出て、その絶対秒が単調増加すること。
///
/// これがそのままサーバーの tempo map へ積まれる。巻き戻すと `TempoMapTimeline::push`
/// に弾かれ、CLAP transport のテンポが追従しなくなる。
#[test]
fn every_wrap_reports_one_monotonic_tempo_change_point() {
    let mut screen = GridSequencerScreen::new(None);
    let now = Instant::now();
    enter_bpm(&mut screen, now, "80-160");

    let mut changes = Vec::new();
    for frame in 0..FRAMES {
        screen.arm_next_cycle_bpm();
        let _ = screen.state.poll_steps(now + FRAME * frame, LOOKAHEAD);
        if let Some(applied) = screen.state.take_applied_cycle_bpm() {
            changes.push(applied);
        }
    }

    assert!(changes.len() > 2, "周ごとに出ていない: {changes:?}");
    let mut last = f64::NEG_INFINITY;
    for applied in &changes {
        assert!(
            applied.at_timeline_seconds > last,
            "変化点が巻き戻った: {last} -> {applied:?}"
        );
        assert!((80.0..=160.0).contains(&applied.bpm), "{applied:?}");
        last = applied.at_timeline_seconds;
    }
}

/// 演奏中の手動 BPM 変更は timeline を張り直さない。
///
/// 張り直す（＝ `BeginTimeline` を送る）とサーバー側で `reset_all(renderers)` が走って
/// 鳴っている音が切れ、サンプルクロックの原点も 0 へ戻る。テンポだけを変えたいので
/// tempo map への追記で済ませること。
#[test]
fn a_manual_bpm_change_while_playing_keeps_the_timeline_running() {
    let mut screen = GridSequencerScreen::new(None);
    let now = Instant::now();
    let ctx = crate::tests::ready_ctx(&[]);
    screen.start(now, &ctx);

    let mut last = f64::NEG_INFINITY;
    for frame in 0..20 {
        for seconds in pump_tempo(&mut screen, now + FRAME * frame) {
            last = last.max(seconds);
        }
    }
    assert!(last > 0.0, "まだ何も組み立てていない");
    let step_before = screen.state.step_index();

    enter_bpm(&mut screen, now + FRAME * 20, "90");
    assert_eq!(screen.bpm(), 90.0);
    assert!(screen.state.is_running(), "演奏が止まっている");
    assert_eq!(
        screen.state.step_index(),
        step_before,
        "再生位置が先頭へ飛んでいる（timeline を張り直した証拠）"
    );

    // 絶対 musical time も繋がったまま。
    for frame in 20..40 {
        for seconds in pump_tempo(&mut screen, now + FRAME * frame) {
            assert!(
                seconds >= last,
                "timeline が巻き戻った: {last} -> {seconds}"
            );
            last = seconds;
        }
    }
}

#[test]
fn the_default_range_keeps_the_previous_fixed_tempo() {
    let mut screen = GridSequencerScreen::new(None);
    let now = Instant::now();
    let ctx = crate::tests::ready_ctx(&[]);
    assert_eq!(screen.bpm_range(), BpmRange::fixed(crate::BPM));
    screen.start(now, &ctx);

    for frame in 0..FRAMES {
        pump_tempo(&mut screen, now + FRAME * frame);
        assert_eq!(screen.bpm(), crate::BPM);
    }
    // 引き直し操作でもテンポは動かない。
    screen.handle_key(key(KeyCode::Char('r'), KeyModifiers::NONE), now, &ctx);
    screen.handle_key(key(KeyCode::Char('R'), KeyModifiers::SHIFT), now, &ctx);
    assert_eq!(screen.bpm(), crate::BPM);
}

#[test]
fn a_manual_bpm_is_never_redrawn_at_a_wrap() {
    let mut screen = GridSequencerScreen::new(None);
    let now = Instant::now();
    enter_bpm(&mut screen, now, "80-160");
    enter_bpm(&mut screen, now, "128");

    for frame in 0..FRAMES {
        pump_tempo(&mut screen, now + FRAME * frame);
        assert_eq!(screen.bpm_mode(), BpmMode::Manual(128.0));
    }
    // 手動へ移っても範囲は覚えておく。
    assert_eq!(screen.bpm_range(), BpmRange::new(80.0, 160.0).unwrap());
}

#[test]
fn entering_the_screen_draws_a_fresh_automatic_bpm() {
    let now = Instant::now();
    let ctx = crate::tests::ready_ctx(&[]);
    let mut screen = GridSequencerScreen::new(None);
    enter_bpm(&mut screen, now, "80-160");
    let first = screen.bpm();

    let moved = (0..32).any(|_| {
        screen.start(now, &ctx);
        screen.bpm() != first
    });
    assert!(moved, "入場時に自動BPMが引き直されていない");
    assert!(screen.state.is_running());
}

#[test]
fn dynamic_step_interval_matches_the_selected_bpm() {
    assert_eq!(
        crate::step_interval_at(120.0),
        std::time::Duration::from_millis(125)
    );
    assert_ne!(crate::step_interval_at(120.0), STEP_INTERVAL);
}
