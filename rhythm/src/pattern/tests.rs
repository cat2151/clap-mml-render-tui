use super::{
    generate_drum, DrumPattern, DrumRole, HatPattern, KickPattern, PercPattern, SnarePattern,
};

/// `steps` step ぶんの Attack 位置だけを取り出す。step 配置の期待値を素直に書くため。
fn attacks(pattern: DrumPattern, steps: usize) -> Vec<usize> {
    generate_drum(pattern, steps)
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
    assert_eq!(attacks(DrumPattern::Kick(KickPattern::Whole), 16), [0]);
    assert!(attacks(DrumPattern::Kick(KickPattern::Silent), 16).is_empty());
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
        attacks(DrumPattern::Perc(PercPattern::Quarter), 16),
        [0, 4, 8, 12]
    );
}

/// 「次の音が鳴るまで伸ばしっぱなし」。最後の音だけ小節末まで伸びる。
#[test]
fn notes_are_held_until_the_next_attack() {
    let hits = generate_drum(DrumPattern::Kick(KickPattern::Quarter), 16);
    assert!(hits.iter().all(|hit| hit.duration_steps == 4));

    let hits = generate_drum(DrumPattern::Snare(SnarePattern::Backbeat), 16);
    assert_eq!(hits[0].duration_steps, 8);
    assert_eq!(hits[1].duration_steps, 4);

    let hits = generate_drum(DrumPattern::Kick(KickPattern::Whole), 16);
    assert_eq!(hits[0].duration_steps, 16);
}

/// 隙間なく敷き詰まっていること。ここが崩れると note off の位置がずれる。
#[test]
fn hits_tile_the_measure_without_gaps() {
    for role in DrumRole::ALL {
        for pattern in DrumPattern::all_for(role) {
            let hits = generate_drum(pattern, 16);
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
            assert!(generate_drum(pattern, 0).is_empty(), "{}", pattern.label());
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

/// ビートの土台は抽選しない。ここが揺れると `r` のたびにリズムが消えることがある。
#[test]
fn only_percussion_is_rerolled() {
    let mut rng = rand::rng();
    for role in [DrumRole::Kick, DrumRole::Snare, DrumRole::HiHat] {
        for _ in 0..16 {
            assert_eq!(
                DrumPattern::random_for(role, &mut rng),
                DrumPattern::default_for(role),
                "{}",
                role.label()
            );
        }
    }
    for _ in 0..16 {
        let pattern = DrumPattern::random_for(DrumRole::Percussion, &mut rng);
        assert_eq!(pattern.role(), DrumRole::Percussion);
        assert!(DrumPattern::all_for(DrumRole::Percussion).contains(&pattern));
    }
}
