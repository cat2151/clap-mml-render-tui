use crate::state::{ChordPlayback, GridState, CHORD_ROW};

fn chord() -> ChordPlayback {
    ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]).unwrap()
}

fn state_with(row_count: usize) -> GridState {
    GridState::with_row_count(row_count)
}

/// 行 `r` は bank 0 では instance `r`、bank 1 では instance `行数 + r` へ写る。
/// この写像がずれると、鳴っている bank と patch を積んだ bank が食い違う。
#[test]
fn rows_map_onto_the_instances_of_their_bank() {
    let mut state = state_with(4);

    assert_eq!(state.bank(), 0);
    assert_eq!(state.instance_id(CHORD_ROW), 0);
    assert_eq!(state.instance_id(3), 3);
    assert_eq!(state.standby_instance_id(CHORD_ROW), 4);
    assert_eq!(state.standby_instance_id(3), 7);

    let rows = state.rows().to_vec();
    state.stage_next_cycle(rows, chord());
    state.mark_pending_ready();
    assert!(state.commit_pending_cycle());

    assert_eq!(state.bank(), 1);
    assert_eq!(state.instance_id(CHORD_ROW), 4);
    assert_eq!(state.instance_id(3), 7);
    assert_eq!(
        state.standby_instance_id(CHORD_ROW),
        0,
        "bank は 2 本で一巡する"
    );
}

#[test]
fn all_four_lanes_share_their_instances_bank_id() {
    let mut state = state_with(2);
    assert_eq!(state.instances()[1].lanes.len(), 4);
    assert!((0..4).all(|_| state.instance_id(1) == 1));
    assert_eq!(state.standby_instance_id(1), 3);

    let instances = state.instances().to_vec();
    state.stage_next_cycle(instances, chord());
    state.mark_pending_ready();
    assert!(state.commit_pending_cycle());
    assert!((0..4).all(|_| state.instance_id(1) == 3));
    assert_eq!(state.standby_instance_id(1), 1);
}

/// 先読みロードの宛先は「待機 bank の instance」でなければならない。
/// 鳴っている bank へ流し込むと、その場で音が差し替わってしまう。
#[test]
fn pending_patches_target_the_standby_bank() {
    let mut state = state_with(2);
    let mut rows = state.rows().to_vec();
    rows[0].patch = Some("Next/A.fxp".to_string());
    rows[1].patch = Some("Next/B.fxp".to_string());
    state.stage_next_cycle(rows, chord());

    assert_eq!(
        state.pending_patches(),
        vec![
            (2, Some("Next/A.fxp".to_string())),
            (3, Some("Next/B.fxp".to_string())),
        ]
    );
}

#[test]
fn there_is_nothing_to_preload_without_a_staged_cycle() {
    assert!(state_with(4).pending_patches().is_empty());
}

/// 先読みが終わったと伝えるまでは差し替えない。
#[test]
fn a_staged_cycle_does_not_swap_until_it_is_marked_ready() {
    let mut state = state_with(2);
    let rows = state.rows().to_vec();
    state.stage_next_cycle(rows, chord());

    assert!(!state.commit_pending_cycle());
    assert_eq!(state.bank(), 0);

    state.mark_pending_ready();
    assert!(state.commit_pending_cycle());
    assert_eq!(state.bank(), 1);
    assert!(!state.has_pending_cycle(), "差し替えたら待ちは空になる");
}

/// HOLD は patch を変えないので、待機 bank へ積み直さず現在 bank 上で進行だけを
/// 差し替える。live edit した patch の instance が境界で変わらないことが重要。
#[test]
fn an_in_place_cycle_commits_without_switching_bank() {
    let mut state = state_with(2);
    let mut rows = state.rows().to_vec();
    rows[0].patch = Some("Held/A.fxp".to_string());

    state.stage_next_cycle_in_place(rows, chord());

    assert!(
        state.pending_patches().is_empty(),
        "HOLD は standby bank へ何もロードしない"
    );
    assert!(state.commit_pending_cycle(), "先読み完了待ちは不要");
    assert_eq!(state.bank(), 0, "HOLD では active bank を維持する");
    assert_eq!(
        state.rows()[0].patch.as_deref(),
        Some("Held/A.fxp"),
        "現在 bank 上で rows を取り込む"
    );
}

/// 抽選し直したら「準備できた」も取り消す。古いロード結果で差し替えないため。
#[test]
fn staging_again_clears_the_ready_flag() {
    let mut state = state_with(2);
    let rows = state.rows().to_vec();
    state.stage_next_cycle(rows.clone(), chord());
    state.mark_pending_ready();

    state.stage_next_cycle(rows, chord());

    assert!(!state.commit_pending_cycle());
}

#[test]
fn discarding_the_pending_cycle_cancels_the_swap() {
    let mut state = state_with(2);
    let rows = state.rows().to_vec();
    state.stage_next_cycle(rows, chord());
    state.mark_pending_ready();

    state.discard_pending_cycle();

    assert!(!state.has_pending_cycle());
    assert!(!state.commit_pending_cycle());
    assert_eq!(state.bank(), 0);
}

/// 差し替え待ちが無いのに「準備できた」と言われても立たない
/// （先読みの完了通知と抽選の取り消しが競合したときの保険）。
#[test]
fn marking_ready_without_a_staged_cycle_does_nothing() {
    let mut state = state_with(2);

    state.mark_pending_ready();

    assert!(!state.commit_pending_cycle());
    assert_eq!(state.bank(), 0);
}
