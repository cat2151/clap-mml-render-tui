use std::time::{Duration, Instant};

use rand::{rngs::StdRng, SeedableRng};

use super::*;

const CHOICES: [u8; 2] = [100, 127];

fn triggers_at(steps: &[usize]) -> TriggerTable {
    vec![std::array::from_fn(|step| steps.contains(&step))]
}

fn lane_after_draw(deadline: Instant, triggers: &TriggerTable) -> MeasureLane {
    let mut lane = MeasureLane::new(1, CHOICES);
    let mut rng = StdRng::seed_from_u64(1);
    lane.draw_measure(deadline, &mut rng, triggers);
    lane
}

#[test]
fn every_cell_is_drawn_from_the_two_choices() {
    let now = Instant::now();

    let lane = lane_after_draw(now, &triggers_at(&[]));

    assert!(lane.values[0].iter().all(|value| CHOICES.contains(value)));
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
}
