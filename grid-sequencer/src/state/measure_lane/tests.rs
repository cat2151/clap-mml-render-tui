use std::time::{Duration, Instant};

use rand::{rngs::StdRng, SeedableRng};

use super::*;
use crate::{ChordPlayback, StepDuration};

const CHOICES: [u8; 2] = [100, 127];

fn triggers_at(steps: &[usize]) -> TriggerTable {
    vec![std::array::from_fn(|step| steps.contains(&step))]
}

fn lane_after_draw(deadline: Instant, triggers: &TriggerTable) -> MeasureLane {
    lane_after_pattern(deadline, MeasurePattern::Random, 1, triggers)
}

fn lane_after_pattern(
    deadline: Instant,
    pattern: MeasurePattern,
    row_count: usize,
    triggers: &TriggerTable,
) -> MeasureLane {
    lane_with_coverage(
        deadline,
        pattern,
        row_count,
        triggers,
        LaneCoverage::SoundingCells,
    )
}

fn lane_with_coverage(
    deadline: Instant,
    pattern: MeasurePattern,
    row_count: usize,
    triggers: &TriggerTable,
    coverage: LaneCoverage,
) -> MeasureLane {
    let spans = vec![RampSpan::WHOLE_MEASURE; row_count];
    let mut lane = MeasureLane::new(row_count, CHOICES, coverage);
    let mut rng = StdRng::seed_from_u64(1);
    lane.draw_measure(deadline, pattern, &mut rng, triggers, &spans);
    lane
}

#[test]
fn every_cell_is_drawn_from_the_two_choices() {
    let now = Instant::now();

    let lane = lane_after_draw(now, &triggers_at(&[]));

    assert!(lane.values[0].iter().all(|value| CHOICES.contains(value)));
}

/// ランプは区間の中のステップ位置で決まるので、区間が同じ行は同じ並びになる。
#[test]
fn a_ramp_fills_rows_with_the_same_span_alike() {
    let now = Instant::now();
    let triggers = vec![[false; GRID_STEPS]; 3];

    let lane = lane_after_pattern(now, MeasurePattern::Ramp { ascending: true }, 3, &triggers);

    assert_eq!(lane.values[0][0], CHOICES[0]);
    assert_eq!(lane.values[0][GRID_STEPS - 1], CHOICES[1]);
    assert!(lane.values.iter().all(|row| *row == lane.values[0]));
}

/// 区間が行ごとに違えば、ランプの並びも行ごとに違う。
#[test]
fn each_row_ramps_inside_its_own_span() {
    let now = Instant::now();
    let triggers = vec![[false; GRID_STEPS]; 2];
    let spans = vec![RampSpan::WHOLE_MEASURE, RampSpan { start: 0, end: 3 }];
    let mut lane = MeasureLane::new(2, CHOICES, LaneCoverage::SoundingCells);
    let mut rng = StdRng::seed_from_u64(1);

    lane.draw_measure(
        now,
        MeasurePattern::Ramp { ascending: true },
        &mut rng,
        &triggers,
        &spans,
    );

    assert_eq!(lane.values[0][3], 105);
    assert_eq!(lane.values[1][3], CHOICES[1]);
}

/// パターンも表示と同じ先読みパイプラインに乗せる。名前だけ先に変わってはいけない。
#[test]
fn the_pattern_label_appears_only_when_the_measure_actually_sounds() {
    let now = Instant::now();
    let deadline = now + Duration::from_millis(100);
    let pattern = MeasurePattern::Ramp { ascending: false };
    let mut lane = lane_after_pattern(deadline, pattern, 1, &triggers_at(&[0]));

    lane.advance_display(now);
    assert_eq!(lane.pattern(), None);

    lane.advance_display(deadline);
    assert_eq!(lane.pattern(), Some(pattern));
}

/// 全stepで送るレーン（CC1）は、鳴らないセルの値も表に出す。
#[test]
fn an_every_step_lane_displays_the_whole_measure() {
    let now = Instant::now();
    let triggers = triggers_at(&[0]);
    let mut lane = lane_with_coverage(
        now,
        MeasurePattern::Random,
        1,
        &triggers,
        LaneCoverage::EveryStep,
    );

    lane.advance_display(now);

    for step in 0..GRID_STEPS {
        assert_eq!(
            lane.display()[0][step],
            Some(lane.values[0][step]),
            "step {step}"
        );
    }
}

/// 非発音セルも抽選しておくのは、小節途中で譜面が変わっても引き直さないため。
#[test]
fn only_the_sounding_cells_are_displayed() {
    let now = Instant::now();
    let mut lane = lane_after_draw(now, &triggers_at(&[0, 4]));

    lane.advance_display(now);

    for step in 0..GRID_STEPS {
        let expected = matches!(step, 0 | 4).then_some(lane.values[0][step]);
        assert_eq!(lane.display()[0][step], expected, "step {step}");
    }
}

#[test]
fn lookahead_does_not_reveal_the_next_measure_before_its_deadline() {
    let now = Instant::now();
    let deadline = now + Duration::from_millis(100);
    let mut lane = lane_after_draw(deadline, &triggers_at(&[0]));

    lane.advance_display(now);
    assert_eq!(lane.display()[0][0], None);

    lane.advance_display(deadline);
    assert_eq!(lane.display()[0][0], Some(lane.values[0][0]));
}

#[test]
fn a_mid_measure_score_change_reuses_the_values_drawn_at_the_measure_start() {
    let now = Instant::now();
    let mut lane = lane_after_draw(now, &triggers_at(&[0]));
    lane.advance_display(now);
    let already_drawn = lane.values[0][3];

    lane.refresh_display(&triggers_at(&[3]));

    assert_eq!(lane.display()[0][0], None);
    assert_eq!(lane.display()[0][3], Some(already_drawn));
}

/// 譜面から引くランプ区間。行の音長ぶんだけ CC1 の終点が velocity より後ろになる。
#[test]
fn the_spans_come_from_the_score_of_each_row() {
    let mut state = GridState::with_row_count(2);
    state.rows[0].cells[6] = true;
    state.rows[0].cells[9] = true;
    state.rows[1].duration = StepDuration::Quarter;
    state.rows[1].cells[0] = true;
    let triggers = state.trigger_table();

    let cc1 = state.cc1_ramp_spans(&triggers);
    let velocity = state.velocity_ramp_spans(&triggers);

    assert_eq!(cc1[0], RampSpan { start: 6, end: 10 });
    assert_eq!(velocity[0], RampSpan { start: 6, end: 9 });
    assert_eq!(cc1[1], RampSpan { start: 0, end: 4 });
    // 音が1つだけの行は幅が潰れ、その音は始点の値で鳴る。
    assert_eq!(velocity[1], RampSpan { start: 0, end: 0 });
}

/// chord mode の和音行は小節いっぱい鳴るので、CC1 のランプも小節全体に張る。
#[test]
fn the_chord_row_spans_the_whole_measure() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    let chord = ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]).unwrap();
    state.set_chord(Some(chord), now);
    let triggers = state.trigger_table();

    let cc1 = state.cc1_ramp_spans(&triggers);

    assert_eq!(cc1[0], RampSpan::WHOLE_MEASURE);
}

#[test]
fn a_restart_clears_both_the_display_and_the_lookahead() {
    let deadline = Instant::now() + Duration::from_millis(100);
    let mut lane = lane_after_draw(deadline, &triggers_at(&[0]));

    lane.reset_for_start();
    lane.advance_display(deadline);

    assert!(lane.display()[0].iter().all(Option::is_none));
    assert_eq!(lane.pattern(), None);
}
