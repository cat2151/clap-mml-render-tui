use std::collections::HashSet;

use rand::{rngs::StdRng, SeedableRng};

use super::*;

const VELOCITY: [u8; 2] = [100, 127];

fn ramp_row(choices: [u8; 2], ascending: bool) -> Vec<u8> {
    (0..GRID_STEPS)
        .map(|step| ramp_value(choices, ascending, step))
        .collect()
}

#[test]
fn a_ramp_starts_and_ends_exactly_on_the_two_choices() {
    let up = ramp_row(VELOCITY, true);
    let down = ramp_row(VELOCITY, false);

    assert_eq!((up[0], up[GRID_STEPS - 1]), (100, 127));
    assert_eq!((down[0], down[GRID_STEPS - 1]), (127, 100));
}

#[test]
fn a_ramp_moves_in_one_direction_only() {
    let up = ramp_row([0, 127], true);
    let down = ramp_row([0, 127], false);

    assert!(up.windows(2).all(|pair| pair[0] <= pair[1]), "{up:?}");
    assert!(down.windows(2).all(|pair| pair[0] >= pair[1]), "{down:?}");
}

/// 中間ステップは線形補間。2値の間の値がそのまま出る。
#[test]
fn the_middle_of_a_ramp_is_interpolated() {
    let up = ramp_row(VELOCITY, true);

    assert_eq!(up[1], 102);
    assert_eq!(up[8], 114);
}

#[test]
fn the_five_plans_are_all_reachable() {
    let mut rng = StdRng::seed_from_u64(1);

    let drawn = (0..200)
        .map(|_| MeasurePlan::draw(&mut rng))
        .collect::<HashSet<_>>();

    assert_eq!(drawn.len(), PLANS.len(), "{drawn:?}");
}

/// 完全ランダムは片方のレーンだけにはかからない。ランプも同様に両レーン同時。
#[test]
fn both_lanes_share_the_same_kind_of_pattern() {
    for plan in PLANS.map(MeasurePlan::from_plan) {
        let both_random =
            plan.cc1 == MeasurePattern::Random && plan.velocity == MeasurePattern::Random;
        let both_ramps = matches!(plan.cc1, MeasurePattern::Ramp { .. })
            && matches!(plan.velocity, MeasurePattern::Ramp { .. });
        assert!(both_random || both_ramps, "{plan:?}");
    }
}
