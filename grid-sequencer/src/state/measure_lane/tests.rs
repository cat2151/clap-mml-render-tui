use std::time::{Duration, Instant};

use rand::{rngs::StdRng, SeedableRng};

use super::*;

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
    let mut lane = MeasureLane::new(row_count, CHOICES);
    let mut rng = StdRng::seed_from_u64(1);
    lane.draw_measure(deadline, pattern, &mut rng, triggers);
    lane
}

#[test]
fn every_cell_is_drawn_from_the_two_choices() {
    let now = Instant::now();

    let lane = lane_after_draw(now, &triggers_at(&[]));

    assert!(lane.values[0].iter().all(|value| CHOICES.contains(value)));
}

/// ランプはステップ位置だけで決まるので、行が違っても同じ並びになる。
#[test]
fn a_ramp_fills_every_row_with_the_same_run() {
    let now = Instant::now();
    let triggers = vec![[false; GRID_STEPS]; 3];

    let lane = lane_after_pattern(now, MeasurePattern::Ramp { ascending: true }, 3, &triggers);

    assert_eq!(lane.values[0][0], CHOICES[0]);
    assert_eq!(lane.values[0][GRID_STEPS - 1], CHOICES[1]);
    assert!(lane.values.iter().all(|row| *row == lane.values[0]));
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

#[test]
fn a_restart_clears_both_the_display_and_the_lookahead() {
    let deadline = Instant::now() + Duration::from_millis(100);
    let mut lane = lane_after_draw(deadline, &triggers_at(&[0]));

    lane.reset_for_start();
    lane.advance_display(deadline);

    assert!(lane.display()[0].iter().all(Option::is_none));
    assert_eq!(lane.pattern(), None);
}
