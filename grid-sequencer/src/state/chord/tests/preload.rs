//! chord progressionの先読み、bank切替、表示deadline。

use super::*;

#[test]
fn starting_a_progression_requests_the_next_cycle_immediately() {
    let now = Instant::now();
    let mut state = chord_only_state();
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);

    assert!(state.take_preload_due());
    assert!(!state.take_preload_due(), "合図は取ったら降ろす");
    assert_eq!(state.chord().unwrap().index(), 0);
}

/// bank の論理切替は MIDI の先読み幅だけ実発音より早い。その時点で空いたように
/// 見える旧 bank をロードすると、境界までまだ鳴っている音を壊す。次の preload は
/// 進行境界の発音 deadline を越えてから合図する。
#[test]
fn a_scheduled_bank_swap_waits_for_its_audio_deadline_before_requesting_the_next_preload() {
    let now = Instant::now();
    let mut state = chord_only_state();
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);
    assert!(state.take_preload_due(), "最初の周の先読み合図を消費する");

    state.stage_next_cycle(state.rows().to_vec(), g_major());
    state.mark_pending_ready();
    let cycle_steps = GRID_STEPS as u64 * 2;
    let cycle_deadline = at_step(now, cycle_steps);

    state.poll_steps(now, cycle_deadline.duration_since(now));

    assert_eq!(
        state.bank(),
        1,
        "スケジューリング側では bank が先に切り替わる"
    );
    assert!(
        !state.take_preload_due(),
        "旧 bank がまだ鳴っている間は次のロードを始めない"
    );

    state.poll_steps(cycle_deadline, Duration::ZERO);

    assert!(
        state.take_preload_due(),
        "進行境界が実際に鳴り始めたら空いた bank を使える"
    );
    assert!(!state.take_preload_due(), "合図は取ったら降ろす");
}

/// MIDI は先読みで次の小節まで組み立てても、コード名と解決音はその小節が実際に
/// 鳴り始めるまで切り替えない。再生ヘッドだけ遅らせると、残り数stepで次のコードが
/// 見えてしまうため、表示内容も同じ締切へ揃える。
#[test]
fn lookahead_keeps_the_displayed_chord_until_its_audio_deadline() {
    let now = Instant::now();
    let mut state = chord_only_state();
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);

    state.poll_steps(now, at_step(now, GRID_STEPS as u64).duration_since(now));

    assert_eq!(
        state.chord().unwrap().index(),
        1,
        "MIDI生成側は次のコードへ進む"
    );
    assert_eq!(
        state.display_chord().unwrap().index(),
        0,
        "発音前のコードを先行表示しない"
    );
    assert_eq!(
        state.display_resolved_note(LaneAddress::new(ARPEGGIO_ROW, 0)),
        Some(60),
        "表示音高も現在聞こえるC chordを保つ"
    );

    state.poll_steps(at_step(now, GRID_STEPS as u64), Duration::ZERO);

    assert_eq!(state.display_chord().unwrap().index(), 1);
    assert_eq!(
        state.display_resolved_note(LaneAddress::new(ARPEGGIO_ROW, 0)),
        Some(65),
        "締切でF chordへ切り替わる"
    );
}

/// ダブルバッファの肝。1周し終えた小節の頭で、待機 bank の grid と進行へ丸ごと
/// 差し替わり、**その場で**新しい和音が鳴る（旧実装のような1小節の無音を挟まない）。
#[test]
fn completing_the_progression_swaps_the_ready_bank_and_keeps_playing() {
    let now = Instant::now();
    let mut state = chord_only_state();
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);
    step_at(&mut state, now);
    let rows = state.rows().to_vec();
    state.stage_next_cycle(rows, g_major());
    state.mark_pending_ready();

    // 2コードぶん（= 進行1周）進める。
    let mut wrapped = Vec::new();
    for step in 1..=(GRID_STEPS as u64 * 2) {
        wrapped = step_at(&mut state, at_step(now, step));
    }

    assert_eq!(state.bank(), 1, "待機 bank へ移る");
    assert_eq!(state.chord().unwrap().degrees(), "V");
    assert_eq!(
        messages(&wrapped),
        vec![
            [0x80, 65, 0],
            [0x80, 69, 0],
            [0x80, 72, 0],
            [0x90, 67, 100],
            [0x90, 71, 100],
            [0x90, 74, 100],
        ],
        "最後のコードを切ってから、間を空けずに新しい和音を鳴らす"
    );
    // 新しい和音は差し替え後の bank の instance へ出す。
    let attacks = wrapped
        .iter()
        .filter(|item| item.message[0] == 0x90)
        .collect::<Vec<_>>();
    assert!(attacks
        .iter()
        .all(|item| item.instance_id == state.instance_id(CHORD_ROW)));
}

#[test]
fn lookahead_keeps_the_old_grid_bank_and_phrase_until_the_cycle_deadline() {
    let now = Instant::now();
    let mut state = chord_only_state();
    state.set_chord(Some(c_major_then_f_major()), now);
    state.instances[2].patch = Some("Old/Lead.fxp".to_string());
    state.display_drawn_now(DrawnPhrases::with_arp(cmrt_arpeggiator::ArpPattern::Up));
    state.start(now);

    let mut next = state.instances().to_vec();
    next[2].patch = Some("Next/Lead.fxp".to_string());
    state.stage_next_cycle_with_drawn(
        next,
        g_major(),
        DrawnPhrases::with_arp(cmrt_arpeggiator::ArpPattern::Down),
    );
    state.mark_pending_ready();
    let cycle_steps = 2 * GRID_STEPS as u64;

    state.poll_steps(now, at_step(now, cycle_steps).duration_since(now));

    assert_eq!(state.bank(), 1, "MIDI生成側は待機bankへ移る");
    assert_eq!(state.instances()[2].patch.as_deref(), Some("Next/Lead.fxp"));
    assert_eq!(state.display_instance_id(2), 2, "表示は旧bankのinstance");
    assert_eq!(
        state.display_instances()[2].patch.as_deref(),
        Some("Old/Lead.fxp")
    );
    assert_eq!(
        state.displayed_drawn_phrases().arp,
        Some(cmrt_arpeggiator::ArpPattern::Up)
    );

    state.poll_steps(at_step(now, cycle_steps), Duration::ZERO);

    assert_eq!(state.display_instance_id(2), 5, "締切で新bankへ移る");
    assert_eq!(
        state.display_instances()[2].patch.as_deref(),
        Some("Next/Lead.fxp")
    );
    assert_eq!(state.display_chord().unwrap().degrees(), "V");
    assert_eq!(
        state.displayed_drawn_phrases().arp,
        Some(cmrt_arpeggiator::ArpPattern::Down)
    );
}

/// 先読みが間に合わなかったら差し替えを見送り、今の grid のまま次の周へ入る。
/// 音を止めないことを最優先する。
#[test]
fn an_unfinished_preload_keeps_the_current_bank() {
    let now = Instant::now();
    let mut state = chord_only_state();
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);
    step_at(&mut state, now);
    let rows = state.rows().to_vec();
    state.stage_next_cycle(rows, g_major());
    // `mark_pending_ready()` を呼ばない = ロードがまだ終わっていない。

    for step in 1..=(GRID_STEPS as u64 * 2) {
        step_at(&mut state, at_step(now, step));
    }

    assert_eq!(state.bank(), 0, "bank は動かさない");
    assert_eq!(state.chord().unwrap().degrees(), "I-IV");
    assert!(state.has_pending_cycle(), "差し替え待ちは次の周へ持ち越す");
}
