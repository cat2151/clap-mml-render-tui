use std::collections::HashSet;

use super::*;

fn candidates(count: usize) -> Vec<String> {
    (0..count)
        .map(|index| format!("Patch{index}.fxp"))
        .collect()
}

fn bag(count: usize, current: Option<&str>) -> PatchBag {
    PatchBag::new(GridPatchPurpose::Note, candidates(count), current)
}

fn next(bag: &mut PatchBag) -> String {
    bag.advance(ListDirection::Next)
        .expect("候補があるので必ず引ける")
        .to_string()
}

fn prev(bag: &mut PatchBag) -> String {
    bag.advance(ListDirection::Prev)
        .expect("履歴があるので必ず戻れる")
        .to_string()
}

#[test]
fn one_bag_hands_out_every_candidate_exactly_once() {
    let mut bag = bag(7, None);
    let drawn = (0..7).map(|_| next(&mut bag)).collect::<Vec<_>>();

    assert_eq!(drawn.iter().collect::<HashSet<_>>().len(), 7);
}

#[test]
fn a_new_bag_is_shuffled_and_refilled_after_the_previous_one_runs_out() {
    let mut bag = bag(7, None);
    let first = (0..7).map(|_| next(&mut bag)).collect::<Vec<_>>();
    let second = (0..7).map(|_| next(&mut bag)).collect::<Vec<_>>();

    assert_eq!(second.iter().collect::<HashSet<_>>().len(), 7);
    // 袋の継ぎ目で同じ patch が並ぶと wheel が効かなく見えるので、そこだけ避ける。
    assert_ne!(first.last(), second.first());
}

#[test]
fn the_wheel_up_walks_back_through_the_patches_it_already_handed_out() {
    let mut bag = bag(7, None);
    let forward = (0..4).map(|_| next(&mut bag)).collect::<Vec<_>>();

    assert_eq!(prev(&mut bag), forward[2]);
    assert_eq!(prev(&mut bag), forward[1]);
    assert_eq!(prev(&mut bag), forward[0]);
}

#[test]
fn walking_back_and_forth_replays_the_same_order_instead_of_drawing_again() {
    let mut bag = bag(7, None);
    let forward = (0..4).map(|_| next(&mut bag)).collect::<Vec<_>>();
    for _ in 0..3 {
        prev(&mut bag);
    }

    let replayed = (0..3).map(|_| next(&mut bag)).collect::<Vec<_>>();
    assert_eq!(replayed, forward[1..]);
}

#[test]
fn the_patch_on_screen_is_the_first_entry_so_the_wheel_up_returns_to_it() {
    let mut bag = bag(7, Some("Current.fxp"));
    next(&mut bag);
    next(&mut bag);

    prev(&mut bag);
    assert_eq!(prev(&mut bag), "Current.fxp");
    // 先頭より前へは戻らない。
    assert_eq!(prev(&mut bag), "Current.fxp");
}

#[test]
fn the_first_draw_never_repeats_the_patch_already_on_screen() {
    for _ in 0..64 {
        let mut bag = bag(3, Some("Patch0.fxp"));
        assert_ne!(next(&mut bag), "Patch0.fxp");
    }
}

#[test]
fn a_patch_that_is_no_longer_a_candidate_still_stays_reachable_as_the_first_entry() {
    let mut bag = bag(3, Some("Retired.fxp"));

    assert!(candidates(3).contains(&next(&mut bag)));
    assert_eq!(prev(&mut bag), "Retired.fxp");
}

#[test]
fn an_empty_catalog_hands_out_nothing() {
    let mut bag = bag(0, None);

    assert_eq!(bag.advance(ListDirection::Next), None);
    assert_eq!(bag.advance(ListDirection::Prev), None);
}

#[test]
fn a_single_candidate_is_handed_out_even_though_it_cannot_avoid_repeating() {
    let mut bag = bag(1, None);

    assert_eq!(next(&mut bag), "Patch0.fxp");
    assert_eq!(next(&mut bag), "Patch0.fxp");
}

#[test]
fn a_bag_is_stale_once_the_role_or_the_candidates_change() {
    let bag = bag(3, None);

    assert!(bag.matches(GridPatchPurpose::Note, &candidates(3)));
    assert!(!bag.matches(GridPatchPurpose::Bass, &candidates(3)));
    assert!(!bag.matches(GridPatchPurpose::Note, &candidates(4)));
}
