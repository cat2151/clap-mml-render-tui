use super::*;

fn assert_one_bag_is_complete(drums: bool, arp: bool, expected_len: usize) {
    let mut bags = PatternBags::default();
    let mut drawn = Vec::new();
    for _ in 0..expected_len {
        let combination = bags.draw(drums, arp).expect("an enabled target");
        assert!(!drawn.contains(&combination), "{combination:?}");
        drawn.push(combination);
    }

    assert_eq!(drawn.len(), expected_len);
    assert_eq!(drawn.len(), PatternCombination::all(drums, arp).len());
}

#[test]
fn target_specific_bags_cover_every_combination_once() {
    assert_one_bag_is_complete(true, false, 6);
    assert_one_bag_is_complete(false, true, 54);
    assert_one_bag_is_complete(true, true, 324);
}

#[test]
fn disabled_targets_do_not_create_or_advance_a_bag() {
    let mut bags = PatternBags::default();

    assert_eq!(bags.draw(false, false), None);
    assert!(bags.bags.is_empty());
}

#[test]
fn each_target_policy_keeps_its_own_bag_progress() {
    let mut bags = PatternBags::default();
    let first_drum = bags.draw(true, false).unwrap();
    let _ = bags.draw(false, true).unwrap();
    let second_drum = bags.draw(true, false).unwrap();

    assert_ne!(first_drum, second_drum);
    assert_eq!(bags.bags.len(), 2);
}

#[test]
fn an_exhausted_bag_refills_from_the_same_combination_list() {
    let mut bags = PatternBags::default();
    for _ in 0..6 {
        let _ = bags.draw(true, false);
    }

    let next = bags.draw(true, false).unwrap();
    assert!(PatternCombination::all(true, false).contains(&next));
}
