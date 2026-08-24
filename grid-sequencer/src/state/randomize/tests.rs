use std::time::{Duration, Instant};

use rand::{rngs::StdRng, SeedableRng};

use super::super::{velocity::normalize_velocity, SCHEDULE_GUARD, STEP_INTERVAL};
use super::*;
use crate::NoteStep;

fn pairs(names: &[&str]) -> Vec<(String, String)> {
    names
        .iter()
        .map(|name| (name.to_string(), name.to_lowercase()))
        .collect()
}

#[test]
fn every_row_gets_a_patch_from_the_loaded_list() {
    let patches = pairs(&["a/Alpha.fxp", "b/Beta.fxp"]);
    let mut state = GridState::silent();
    state.randomize_all(Instant::now(), &patches);
    assert!(state.instances.iter().all(|instance| {
        instance
            .patch
            .as_deref()
            .is_some_and(|patch| patches.iter().any(|(display, _)| display == patch))
    }));
}

#[test]
fn existing_patches_are_kept_while_the_patch_list_is_unavailable() {
    let mut state = GridState::silent();
    state.instances[0].patch = Some("kept/Patch.fxp".to_string());
    state.randomize_all(Instant::now(), &[]);
    assert_eq!(state.instances[0].patch.as_deref(), Some("kept/Patch.fxp"));
    assert_eq!(state.instances[1].patch, None);
}

#[test]
fn notes_stay_inside_the_generated_range() {
    let mut state = GridState::silent();
    state.randomize_all(Instant::now(), &[]);
    assert!(state
        .instances
        .iter()
        .flat_map(|instance| &instance.lanes)
        .all(|lane| (RANDOM_NOTE_MIN..=RANDOM_NOTE_MAX).contains(&lane.base_note)));
}

#[test]
fn generated_patterns_reach_one_two_and_four_step_notes_without_orphan_ties() {
    let mut rng = StdRng::seed_from_u64(7);
    let mut lengths = std::collections::BTreeSet::new();
    for _ in 0..256 {
        let pattern = random_pattern(&mut rng);
        for step in 0..GRID_STEPS {
            match pattern.step(step).unwrap() {
                NoteStep::Attack => {
                    lengths.insert(pattern.attack_len(step).unwrap());
                }
                NoteStep::Tie => assert!(
                    step > 0 && pattern.step(step - 1) != Some(NoteStep::Rest),
                    "orphan Tie at step {step}"
                ),
                NoteStep::Rest => {}
            }
        }
    }
    assert!(lengths.contains(&1));
    assert!(lengths.contains(&2));
    assert!(lengths.contains(&4));
}

#[test]
fn sounding_notes_are_silenced_so_they_do_not_hang() {
    let now = Instant::now();
    let mut state = GridState::silent();
    state.instances[0].base_note = 64;
    state.instances[0].pattern.draw_span(0, 0);
    state.start(now);
    let scheduled = state.poll_steps(now, Duration::ZERO);
    assert_eq!(messages_of(&scheduled), vec![[0x90, 64, 100]]);

    let silenced = state.randomize_all(now, &[]);

    assert_eq!(messages_of(&silenced), vec![[0x80, 64, 0]]);
    // 先読みで送信済みの note on より後ろへ置く（先回りして止めない）。
    assert_eq!(silenced[0].ahead, SCHEDULE_GUARD);
}

/// 先読みで既に送ってしまったステップより後ろへ note off が回ることを見る。
#[test]
fn silencing_waits_for_the_notes_already_sent_ahead() {
    let now = Instant::now();
    let mut state = GridState::silent();
    state.instances[0].base_note = 64;
    state.instances[0].pattern.draw_span(0, 3);
    state.start(now);
    // 2ステップ先まで送信済みにする。
    state.poll_steps(now, STEP_INTERVAL * 2);

    let silenced = state.randomize_all(now, &[]);

    assert_eq!(silenced[0].ahead, STEP_INTERVAL * 2 + SCHEDULE_GUARD);
}

/// 音色ロードを走らせないため、patch だけは引き直さない。
#[test]
fn keeping_patches_rerolls_everything_except_the_patch() {
    let mut state = GridState::silent();
    for instance in &mut state.instances {
        instance.patch = Some("kept/Patch.fxp".to_string());
        for lane in &mut instance.lanes {
            lane.pattern = NotePattern::default();
        }
    }

    state.randomize_keeping_patches(Instant::now());

    assert!(state
        .instances
        .iter()
        .all(|instance| instance.patch.as_deref() == Some("kept/Patch.fxp")));
    assert!(state
        .instances
        .iter()
        .flat_map(|instance| &instance.lanes)
        .all(|lane| (RANDOM_NOTE_MIN..=RANDOM_NOTE_MAX).contains(&lane.base_note)));
    assert!(
        state
            .instances
            .iter()
            .flat_map(|instance| &instance.lanes)
            .any(|lane| lane.pattern.steps().contains(&NoteStep::Attack)),
        "patch 以外は引き直すので、どこかにAttackが生成される"
    );
}

/// `randomize_all` と違って音色切替の `stop_live_all()` が後ろに続かないので、
/// この note off を送らないと音が鳴りっぱなしになる。
#[test]
fn keeping_patches_still_silences_sounding_notes() {
    let now = Instant::now();
    let mut state = GridState::silent();
    state.instances[0].base_note = 64;
    state.instances[0].pattern.draw_span(0, 0);
    state.start(now);
    state.poll_steps(now, Duration::ZERO);

    let silenced = state.randomize_keeping_patches(now);

    assert_eq!(messages_of(&silenced), vec![[0x80, 64, 0]]);
    assert_eq!(silenced[0].ahead, SCHEDULE_GUARD);
}

/// CC1 を除き、抽選値の velocity は既定値へ均して譜面だけを見る。
fn messages_of(scheduled: &[GridScheduledMessage]) -> Vec<[u8; 3]> {
    scheduled
        .iter()
        .filter(|item| item.message[0] != 0xB0)
        .map(|item| normalize_velocity(item.message))
        .collect()
}

#[test]
fn swing_is_drawn_inside_the_valid_range_for_every_row() {
    let mut state = GridState::silent();
    state.randomize_all(Instant::now(), &[]);
    assert!(
        state
            .instances
            .iter()
            .all(|instance| (SWING_MIN..=SWING_MAX).contains(&instance.swing)),
        "{:?}",
        state
            .instances
            .iter()
            .map(|instance| instance.swing)
            .collect::<Vec<_>>()
    );
}

/// SWING が OFF の周は swing を据え置く。譜面だけ引き直しても groove は変わらない。
#[test]
fn swing_is_held_when_the_policy_says_so() {
    let mut instances = vec![GridInstance::new(0); 4];
    for (index, instance) in instances.iter_mut().enumerate() {
        instance.swing = SWING_MIN + index as u8;
    }

    randomize_instance_slice(
        &mut instances,
        &[],
        CycleRandom {
            swing: false,
            ..CycleRandom::ALL
        },
        None,
        None,
    );

    let swings = instances
        .iter()
        .map(|instance| instance.swing)
        .collect::<Vec<_>>();
    assert_eq!(swings, vec![50, 51, 52, 53]);
}

/// drum 行も対象。hi-hat の16分こそ跳ねてほしい行なので、役割で除外しない。
#[test]
fn drum_rows_get_swing_too() {
    let mut instances = vec![GridInstance::new(0); crate::FULL_DRUM_TRACK_COUNT];
    drum::apply_drum_roles(&mut instances);
    assert!(instances.iter().any(|instance| instance.drum.is_some()));

    let mut bags = super::super::pattern_bag::PatternBags::default();
    let patterns = bags.draw(true, false, false);
    randomize_instance_slice(&mut instances, &[], CycleRandom::ALL, None, patterns);

    assert!(instances
        .iter()
        .filter(|instance| instance.drum.is_some())
        .all(|instance| (SWING_MIN..=SWING_MAX).contains(&instance.swing)));
}

/// 2track chord modeにはChord行とBass行だけがあり、Arpeggio行は無い。
/// Bassだけでも専用bagから音型を引けないと、次cycleの抽選時にpanicする。
#[test]
fn two_track_chord_mode_draws_a_bass_pattern_without_an_arpeggio_row() {
    let chord = ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]).unwrap();
    let mut state = GridState::with_instance_count(2);
    let patterns = state
        .draw_pattern_combination(CycleRandom::ALL, true)
        .expect("the existing bass row has a pattern bag");
    assert!(patterns.bass_pattern().is_some());
    assert_eq!(patterns.arp_pattern(), None);

    let drawn = randomize_instance_slice(
        &mut state.instances,
        &[],
        CycleRandom::ALL,
        Some(&chord),
        Some(patterns),
    );

    assert!(drawn.bass.is_some());
    assert_eq!(drawn.arp, None);
}
