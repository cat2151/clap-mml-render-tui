use super::{
    generate_drum, DrumPattern, DrumPatternCombination, DrumRole, HatPattern, KickPattern,
    PercPattern, SnarePattern,
};
use rand::{rngs::StdRng, SeedableRng};

/// `steps` step ぶんの Attack 位置だけを取り出す。step 配置の期待値を素直に書くため。
fn attacks(pattern: DrumPattern, steps: usize) -> Vec<usize> {
    generate_drum(pattern, steps, &mut rand::rng())
        .into_iter()
        .map(|hit| hit.step)
        .collect()
}

#[test]
fn each_pattern_places_attacks_where_documented() {
    assert_eq!(
        attacks(DrumPattern::Kick(KickPattern::Quarter), 16),
        [0, 4, 8, 12]
    );
    assert_eq!(
        attacks(DrumPattern::Kick(KickPattern::OneAndThreeOffbeat), 16),
        [0, 10]
    );
    // 裏拍の八分ではなく2・4拍。
    assert_eq!(
        attacks(DrumPattern::Snare(SnarePattern::Backbeat), 16),
        [4, 12]
    );
    assert_eq!(
        attacks(DrumPattern::Hat(HatPattern::Eighth), 16),
        [0, 2, 4, 6, 8, 10, 12, 14]
    );
    assert_eq!(
        attacks(DrumPattern::Hat(HatPattern::Sixteenth), 16),
        (0..16).collect::<Vec<_>>()
    );
    assert_eq!(
        attacks(DrumPattern::Hat(HatPattern::OffbeatQuarter), 16),
        [2, 6, 10, 14]
    );
}

/// 「次の音が鳴るまで伸ばしっぱなし」。最後の音だけ小節末まで伸びる。
#[test]
fn notes_are_held_until_the_next_attack() {
    let hits = generate_drum(
        DrumPattern::Kick(KickPattern::Quarter),
        16,
        &mut rand::rng(),
    );
    assert!(hits.iter().all(|hit| hit.duration_steps == 4));

    let hits = generate_drum(
        DrumPattern::Snare(SnarePattern::Backbeat),
        16,
        &mut rand::rng(),
    );
    assert_eq!(hits[0].duration_steps, 8);
    assert_eq!(hits[1].duration_steps, 4);
}

/// 隙間なく敷き詰まっていること。ここが崩れると note off の位置がずれる。
#[test]
fn hits_tile_the_measure_without_gaps() {
    for role in DrumRole::ALL {
        for pattern in DrumPattern::all_for(role) {
            let hits = generate_drum(pattern, 16, &mut rand::rng());
            let Some(first) = hits.first() else {
                continue;
            };
            let covered = hits.iter().map(|hit| hit.duration_steps).sum::<usize>();
            assert_eq!(covered, 16 - first.step, "{}", pattern.label());
            for pair in hits.windows(2) {
                assert_eq!(
                    pair[0].step + pair[0].duration_steps,
                    pair[1].step,
                    "{}",
                    pattern.label()
                );
            }
        }
    }
}

#[test]
fn zero_steps_generates_nothing() {
    for role in DrumRole::ALL {
        for pattern in DrumPattern::all_for(role) {
            assert!(
                generate_drum(pattern, 0, &mut rand::rng()).is_empty(),
                "{}",
                pattern.label()
            );
        }
    }
}

#[test]
fn next_and_previous_walk_each_role_list_and_wrap() {
    for role in DrumRole::ALL {
        let all = DrumPattern::all_for(role);
        let mut pattern = all[0];
        for expected in all.iter().skip(1).chain(&all[..1]) {
            pattern = pattern.next();
            assert_eq!(pattern, *expected, "{}", role.label());
        }
        assert_eq!(pattern, all[0]);

        for expected in all.iter().rev() {
            assert_eq!(pattern.previous(), *expected, "{}", role.label());
            pattern = pattern.previous();
        }
    }
}

/// 送りで役割をまたがないこと。またぐと wheel 1回で patch と噛み合わない型が入る。
#[test]
fn cycling_stays_inside_the_role() {
    for role in DrumRole::ALL {
        let mut pattern = DrumPattern::default_for(role);
        for _ in 0..DrumPattern::all_for(role).len() + 1 {
            pattern = pattern.next();
            assert_eq!(pattern.role(), role);
        }
    }
}

#[test]
fn the_default_is_the_head_of_each_role_list() {
    for role in DrumRole::ALL {
        assert_eq!(
            DrumPattern::default_for(role),
            DrumPattern::all_for(role)[0],
            "{}",
            role.label()
        );
        assert_eq!(DrumPattern::default_for(role).role(), role);
    }
}

#[test]
fn labels_are_unique_inside_each_role() {
    for role in DrumRole::ALL {
        let mut labels = DrumPattern::all_for(role)
            .iter()
            .map(|pattern| pattern.label())
            .collect::<Vec<_>>();
        let count = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), count, "{}", role.label());
    }
}

#[test]
fn simple_drum_lists_omit_silent() {
    assert_eq!(
        DrumPattern::all_for(DrumRole::Kick),
        [
            DrumPattern::Kick(KickPattern::Quarter),
            DrumPattern::Kick(KickPattern::OneAndThreeOffbeat),
        ]
    );
    assert_eq!(
        DrumPattern::all_for(DrumRole::Snare),
        [DrumPattern::Snare(SnarePattern::Backbeat)]
    );
    assert_eq!(
        DrumPattern::all_for(DrumRole::HiHat),
        [
            DrumPattern::Hat(HatPattern::Eighth),
            DrumPattern::Hat(HatPattern::Sixteenth),
            DrumPattern::Hat(HatPattern::OffbeatQuarter),
        ]
    );
}

#[test]
fn combinations_are_the_cartesian_product_of_every_role_list() {
    let combinations = DrumPatternCombination::all();
    let expected_count = DrumRole::ALL
        .iter()
        .map(|role| DrumPattern::all_for(*role).len())
        .product::<usize>();

    assert_eq!(combinations.len(), expected_count);
    for combination in &combinations {
        for role in DrumRole::ALL {
            assert!(
                DrumPattern::all_for(role).contains(&combination.pattern_for(role)),
                "{}: {combination:?}",
                role.label()
            );
        }
    }
    for (index, combination) in combinations.iter().enumerate() {
        assert!(!combinations[..index].contains(combination));
    }
}

#[test]
fn random_percussion_draws_one_to_three_unique_attacks() {
    let mut rng = StdRng::seed_from_u64(7);
    let pattern = DrumPattern::Perc(PercPattern::Random);

    for _ in 0..128 {
        let hits = generate_drum(pattern, 16, &mut rng);
        assert!((1..=3).contains(&hits.len()), "{hits:?}");
        assert!(hits.windows(2).all(|pair| pair[0].step < pair[1].step));
        assert!(hits.iter().all(|hit| hit.step < 16));
    }
}

#[test]
fn random_percussion_holds_until_the_next_attack_or_measure_end() {
    let mut rng = StdRng::seed_from_u64(11);
    let hits = generate_drum(DrumPattern::Perc(PercPattern::Random), 16, &mut rng);

    for pair in hits.windows(2) {
        assert_eq!(pair[0].step + pair[0].duration_steps, pair[1].step);
    }
    let last = hits
        .last()
        .expect("random always draws at least one attack");
    assert_eq!(last.step + last.duration_steps, 16);
}

#[test]
fn random_percussion_clamps_its_note_count_to_short_measures() {
    let mut rng = StdRng::seed_from_u64(13);
    let pattern = DrumPattern::Perc(PercPattern::Random);

    assert!(generate_drum(pattern, 0, &mut rng).is_empty());
    for steps in 1..=2 {
        for _ in 0..16 {
            let hits = generate_drum(pattern, steps, &mut rng);
            assert!((1..=steps).contains(&hits.len()), "steps={steps} {hits:?}");
            assert!(hits.iter().all(|hit| hit.step < steps));
        }
    }
}

#[test]
fn random_is_the_only_percussion_pattern() {
    assert_eq!(
        DrumPattern::all_for(DrumRole::Percussion),
        [DrumPattern::Perc(PercPattern::Random)]
    );
    assert_eq!(PercPattern::default(), PercPattern::Random);
    assert_eq!(PercPattern::Random.label(), "Random");
}
