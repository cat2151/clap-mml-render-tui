use super::*;
use crate::session_state::KeyboardSessionState;
use crate::{KeyboardState, NotePlaybackMode};

const NOTE_ON_C: [u8; 3] = [0x90, 60, 100];
const NOTE_OFF_C: [u8; 3] = [0x80, 60, 0];

fn tick(at: Instant, messages: Vec<[u8; 3]>) -> Option<PeriodicTick> {
    Some(PeriodicTick { at, messages })
}

fn ids(first: TimelineId) -> impl FnMut() -> TimelineId {
    let mut next = first;
    move || {
        let id = next;
        next += 1;
        id
    }
}

fn scheduled(sends: &[TimelineSend]) -> Vec<(TimelineId, f64, [u8; 3])> {
    sends
        .iter()
        .filter_map(|send| match send {
            TimelineSend::Scheduled(events) => Some(events),
            _ => None,
        })
        .flatten()
        .map(|event| (event.timeline_id, event.timeline_seconds, event.message))
        .collect()
}

#[test]
fn first_send_begins_the_timeline_before_immediate_and_scheduled() {
    let origin = Instant::now();
    let mut timeline = PeriodicTimeline::default();
    let mut next_id = ids(7);

    let sends = timeline.plan(
        origin,
        vec![NOTE_ON_C],
        tick(origin + Duration::from_millis(250), vec![NOTE_OFF_C]),
        &mut next_id,
    );

    assert_eq!(sends.len(), 3);
    assert_eq!(sends[0], TimelineSend::Begin(7));
    assert_eq!(sends[1], TimelineSend::Immediate(vec![NOTE_ON_C]));
    assert_eq!(scheduled(&sends), vec![(7, 0.25, NOTE_OFF_C)]);
}

#[test]
fn later_ticks_are_scheduled_at_deadline_seconds_regardless_of_send_time() {
    let origin = Instant::now();
    let mut timeline = PeriodicTimeline::default();
    let mut next_id = ids(1);
    timeline.plan(origin, Vec::new(), None, &mut next_id);

    // 送った時刻（deadline - LOOKAHEAD から遅れたもの）は秒に影響しない
    for (k, late_ms) in [(1_u64, 0_u64), (2, 37), (3, 120), (40, 5)] {
        let at = origin + Duration::from_millis(250 * k);
        let sent_at = at - LOOKAHEAD + Duration::from_millis(late_ms);
        let sends = timeline.plan(sent_at, Vec::new(), tick(at, vec![NOTE_ON_C]), &mut next_id);
        assert_eq!(sends.len(), 1, "no Begin while the timeline is active");
        let events = scheduled(&sends);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, 1);
        assert!(
            (events[0].1 - 0.25 * k as f64).abs() < 1e-9,
            "tick {k}: {}",
            events[0].1
        );
    }
}

#[test]
fn every_message_of_a_tick_shares_the_same_timeline_second() {
    let origin = Instant::now();
    let mut timeline = PeriodicTimeline::default();
    let cc = [0xB0, 1, 64];
    let sends = timeline.plan(
        origin,
        Vec::new(),
        tick(
            origin + Duration::from_millis(500),
            vec![cc, NOTE_OFF_C, NOTE_ON_C],
        ),
        ids(3),
    );

    assert_eq!(
        scheduled(&sends),
        vec![(3, 0.5, cc), (3, 0.5, NOTE_OFF_C), (3, 0.5, NOTE_ON_C)]
    );
}

#[test]
fn empty_tick_and_empty_immediate_send_nothing_once_active() {
    let origin = Instant::now();
    let mut timeline = PeriodicTimeline::default();
    timeline.plan(origin, Vec::new(), None, ids(1));

    let sends = timeline.plan(
        origin + Duration::from_millis(50),
        Vec::new(),
        tick(origin + Duration::from_millis(250), Vec::new()),
        ids(2),
    );
    assert!(sends.is_empty(), "{sends:?}");
}

#[test]
fn cancel_begins_a_new_timeline_first_and_rebases_seconds() {
    let origin = Instant::now();
    let mut timeline = PeriodicTimeline::default();
    let mut next_id = ids(1);
    timeline.plan(origin, Vec::new(), None, &mut next_id);

    let restarted = origin + Duration::from_millis(1_000);
    timeline.request_cancel();
    let sends = timeline.plan(restarted, vec![NOTE_OFF_C, NOTE_ON_C], None, &mut next_id);
    assert_eq!(
        sends,
        vec![
            TimelineSend::Begin(2),
            TimelineSend::Immediate(vec![NOTE_OFF_C, NOTE_ON_C]),
        ]
    );

    let sends = timeline.plan(
        restarted + Duration::from_millis(60),
        Vec::new(),
        tick(restarted + Duration::from_millis(250), vec![NOTE_ON_C]),
        &mut next_id,
    );
    assert_eq!(scheduled(&sends), vec![(2, 0.25, NOTE_ON_C)]);
}

#[test]
fn a_cancel_request_is_consumed_by_one_begin() {
    let origin = Instant::now();
    let mut timeline = PeriodicTimeline::default();
    let mut next_id = ids(1);
    timeline.plan(origin, Vec::new(), None, &mut next_id);
    timeline.request_cancel();
    timeline.plan(origin, Vec::new(), None, &mut next_id);

    let sends = timeline.plan(origin, vec![NOTE_ON_C], None, &mut next_id);
    assert_eq!(sends, vec![TimelineSend::Immediate(vec![NOTE_ON_C])]);
}

#[test]
fn forgotten_timeline_is_begun_again_on_the_next_send() {
    let origin = Instant::now();
    let mut timeline = PeriodicTimeline::default();
    let mut next_id = ids(1);
    timeline.plan(origin, Vec::new(), None, &mut next_id);
    timeline.forget();

    let sends = timeline.plan(origin, vec![NOTE_ON_C], None, &mut next_id);
    assert_eq!(
        sends,
        vec![
            TimelineSend::Begin(2),
            TimelineSend::Immediate(vec![NOTE_ON_C]),
        ]
    );
}

#[test]
fn tick_before_the_origin_is_clamped_to_zero_seconds() {
    let at = Instant::now();
    let mut timeline = PeriodicTimeline::default();
    let sends = timeline.plan(
        at + Duration::from_millis(30),
        Vec::new(),
        tick(at, vec![NOTE_ON_C]),
        ids(1),
    );
    assert_eq!(scheduled(&sends), vec![(1, 0.0, NOTE_ON_C)]);
}

#[test]
fn stop_discards_reservations_before_note_offs_then_forgets() {
    let origin = Instant::now();
    let mut timeline = PeriodicTimeline::default();
    let mut next_id = ids(1);
    timeline.plan(origin, Vec::new(), None, &mut next_id);

    let sends = timeline.plan_stop(vec![NOTE_OFF_C], &mut next_id);
    assert_eq!(
        sends,
        vec![
            TimelineSend::Begin(2),
            TimelineSend::Immediate(vec![NOTE_OFF_C]),
        ]
    );
    // 次の入場では張り直す
    let sends = timeline.plan(origin, Vec::new(), None, &mut next_id);
    assert_eq!(sends, vec![TimelineSend::Begin(3)]);
}

#[test]
fn stop_without_a_timeline_sends_only_note_offs() {
    let mut timeline = PeriodicTimeline::default();
    let sends = timeline.plan_stop(vec![NOTE_OFF_C], || panic!("no Begin expected"));
    assert_eq!(sends, vec![TimelineSend::Immediate(vec![NOTE_OFF_C])]);
}

// 状態機械を LOOKAHEAD 先で poll したときの送信列。restart の最初の音は即時、
// 以降の tick は anchor からの秒で予約される。
#[test]
fn arp_polled_ahead_schedules_each_tick_on_the_anchor_grid() {
    let anchor = Instant::now();
    let mut state = KeyboardState::from_session(KeyboardSessionState {
        note_playback_mode: NotePlaybackMode::Arp,
        repeat_chords: vec![vec![60, 64, 67]],
        ..KeyboardSessionState::default()
    });
    let mut timeline = PeriodicTimeline::default();
    let mut next_id = ids(1);
    let refresh = state.take_pending_refresh_messages(anchor);
    let first = state.poll_periodic_tick(anchor + LOOKAHEAD);
    let sends = timeline.plan(anchor, refresh, first, &mut next_id);
    assert_eq!(
        sends,
        vec![
            TimelineSend::Begin(1),
            TimelineSend::Immediate(vec![NOTE_ON_C]),
        ]
    );

    let mut seconds = Vec::new();
    let mut now = anchor;
    for frame in 1..=80_u64 {
        // 不揃いな frame 間隔（20〜120ms）
        now += Duration::from_millis(20 + (frame * 37) % 101);
        let tick = state.poll_periodic_tick(now + LOOKAHEAD);
        let sends = timeline.plan(now, Vec::new(), tick, &mut next_id);
        assert!(!sends
            .iter()
            .any(|send| matches!(send, TimelineSend::Begin(_))));
        seconds.extend(
            scheduled(&sends)
                .into_iter()
                .filter(|(_, _, message)| message[0] == 0x90)
                .map(|(_, second, _)| second),
        );
    }
    assert!(seconds.len() >= 10, "{seconds:?}");
    for (k, second) in seconds.iter().enumerate() {
        let expected = 0.25 * (k + 1) as f64;
        assert!((second - expected).abs() < 1e-9, "tick {}: {second}", k + 1);
    }
}

#[test]
fn stop_keeps_periodic_sending_stopped_until_restart() {
    let origin = Instant::now();
    let mut timeline = PeriodicTimeline::default();
    let mut next_id = ids(1);
    timeline.plan(origin, Vec::new(), None, &mut next_id);
    assert!(!timeline.is_stopped());

    timeline.plan_stop(Vec::new(), &mut next_id);
    assert!(timeline.is_stopped());
    // 接続が切れて忘れても、止めたままにする
    timeline.forget();
    assert!(timeline.is_stopped());

    timeline.restart();
    assert!(!timeline.is_stopped());
    let sends = timeline.plan(origin, Vec::new(), None, &mut next_id);
    assert_eq!(sends, vec![TimelineSend::Begin(3)]);
}

// MML overlay を開いて閉じる往復: 出るときに arp を止め、戻った最初の送信で
// 新しい timeline を張ってから arp を頭から鳴らし直し、以後の tick はその timeline の格子に乗る。
#[test]
fn leaving_and_resuming_restarts_arp_on_a_new_timeline() {
    let entered = Instant::now();
    let mut state = KeyboardState::from_session(KeyboardSessionState {
        note_playback_mode: NotePlaybackMode::Arp,
        repeat_chords: vec![vec![60, 64, 67]],
        ..KeyboardSessionState::default()
    });
    let mut timeline = PeriodicTimeline::default();
    let mut next_id = ids(1);
    let refresh = state.take_pending_refresh_messages(entered);
    timeline.plan(entered, refresh, None, &mut next_id);
    let mut now = entered;
    for _ in 0..3 {
        now += Duration::from_millis(250);
        let tick = state.poll_periodic_tick(now + LOOKAHEAD);
        timeline.plan(now, Vec::new(), tick, &mut next_id);
    }

    // 出る（overlay を開く）
    let note_offs = state.take_leave_messages();
    let sends = timeline.plan_stop(note_offs, &mut next_id);
    assert_eq!(sends[0], TimelineSend::Begin(2));
    assert!(timeline.is_stopped());
    assert_eq!(state.periodic_anchor(), None);

    // 戻る（overlay を閉じる）
    let resumed = now + Duration::from_secs(3);
    timeline.restart();
    let refresh = state.take_pending_refresh_messages(resumed);
    let first = state.poll_periodic_tick(resumed + LOOKAHEAD);
    let sends = timeline.plan(resumed, refresh, first, &mut next_id);
    assert_eq!(
        sends,
        vec![
            TimelineSend::Begin(3),
            TimelineSend::Immediate(vec![NOTE_ON_C]),
        ]
    );
    assert_eq!(state.periodic_anchor(), Some(resumed));

    let tick = state.poll_periodic_tick(resumed + Duration::from_millis(60) + LOOKAHEAD);
    let sends = timeline.plan(
        resumed + Duration::from_millis(60),
        Vec::new(),
        tick,
        &mut next_id,
    );
    let note_ons: Vec<_> = scheduled(&sends)
        .into_iter()
        .filter(|(_, _, message)| message[0] == 0x90)
        .collect();
    assert_eq!(note_ons.len(), 1, "{sends:?}");
    assert_eq!(note_ons[0].0, 3);
    assert!((note_ons[0].1 - 0.25).abs() < 1e-9, "{note_ons:?}");
}
