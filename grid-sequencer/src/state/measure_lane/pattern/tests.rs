use std::collections::HashSet;

use rand::{rngs::StdRng, SeedableRng};

use super::*;

const VELOCITY: [u8; 2] = [100, 127];

fn ramp_row(choices: [u8; 2], ascending: bool) -> Vec<u8> {
    span_row(choices, ascending, RampSpan::WHOLE_MEASURE)
}

fn span_row(choices: [u8; 2], ascending: bool, span: RampSpan) -> Vec<u8> {
    (0..GRID_STEPS)
        .map(|step| ramp_value(choices, ascending, step, span))
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

/// 区間を絞っても、その両端で2値ちょうどに届く。中央に2音しかない行の救済。
#[test]
fn a_narrow_span_still_reaches_both_choices() {
    let up = span_row(VELOCITY, true, RampSpan { start: 6, end: 9 });

    assert_eq!((up[6], up[9]), (100, 127));
    assert!(up[7] > 100 && up[7] < 127, "{up:?}");
}

/// 区間の外は端の値でクランプする。先頭の休符は始点の値、末尾は終点の値。
#[test]
fn the_values_outside_the_span_are_clamped_to_its_ends() {
    let up = span_row(VELOCITY, true, RampSpan { start: 6, end: 9 });

    assert!(up[..6].iter().all(|value| *value == 100), "{up:?}");
    assert!(up[10..].iter().all(|value| *value == 127), "{up:?}");
}

/// 幅が潰れた区間は始点の値で一定。音が1つしかない行の velocity がこれ。
#[test]
fn a_collapsed_span_holds_the_starting_choice() {
    let up = span_row(VELOCITY, true, RampSpan { start: 7, end: 7 });
    let down = span_row(VELOCITY, false, RampSpan { start: 7, end: 7 });

    assert!(up.iter().all(|value| *value == 100), "{up:?}");
    assert!(down.iter().all(|value| *value == 127), "{down:?}");
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
