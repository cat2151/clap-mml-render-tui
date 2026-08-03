use std::time::{Duration, Instant};

use super::super::{SCHEDULE_GUARD, STEP_INTERVAL};
use super::*;

fn pairs(names: &[&str]) -> Vec<(String, String)> {
    names
        .iter()
        .map(|name| (name.to_string(), name.to_lowercase()))
        .collect()
}

#[test]
fn every_row_gets_a_patch_from_the_loaded_list() {
    let patches = pairs(&["a/Alpha.fxp", "b/Beta.fxp"]);
    let mut state = GridState::default();
    state.randomize_all(Instant::now(), &patches);
    assert!(state.rows.iter().all(|row| {
        row.patch
            .as_deref()
            .is_some_and(|patch| patches.iter().any(|(display, _)| display == patch))
    }));
}

#[test]
fn existing_patches_are_kept_while_the_patch_list_is_unavailable() {
    let mut state = GridState::default();
    state.rows[0].patch = Some("kept/Patch.fxp".to_string());
    state.randomize_all(Instant::now(), &[]);
    assert_eq!(state.rows[0].patch.as_deref(), Some("kept/Patch.fxp"));
    assert_eq!(state.rows[1].patch, None);
}

#[test]
fn notes_stay_inside_the_generated_range() {
    let mut state = GridState::default();
    state.randomize_all(Instant::now(), &[]);
    assert!(state
        .rows
        .iter()
        .all(|row| (RANDOM_NOTE_MIN..=RANDOM_NOTE_MAX).contains(&row.note)));
}

#[test]
fn sounding_notes_are_silenced_so_they_do_not_hang() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.rows[0].note = 64;
    state.rows[0].cells[0] = true;
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
    let mut state = GridState::default();
    state.rows[0].note = 64;
    state.rows[0].duration = StepDuration::Quarter;
    state.rows[0].cells[0] = true;
    state.start(now);
    // 2ステップ先まで送信済みにする。
    state.poll_steps(now, STEP_INTERVAL * 2);

    let silenced = state.randomize_all(now, &[]);

    assert_eq!(silenced[0].ahead, STEP_INTERVAL * 2 + SCHEDULE_GUARD);
}

/// 音色ロードを走らせないため、patch だけは引き直さない。
#[test]
fn keeping_patches_rerolls_everything_except_the_patch() {
    let mut state = GridState::default();
    for row in &mut state.rows {
        row.patch = Some("kept/Patch.fxp".to_string());
        row.cells = [false; GRID_STEPS];
    }

    state.randomize_keeping_patches(Instant::now());

    assert!(state
        .rows
        .iter()
        .all(|row| row.patch.as_deref() == Some("kept/Patch.fxp")));
    assert!(state
        .rows
        .iter()
        .all(|row| (RANDOM_NOTE_MIN..=RANDOM_NOTE_MAX).contains(&row.note)));
    assert!(
        state
            .rows
            .iter()
            .any(|row| row.cells.iter().any(|cell| *cell)),
        "patch 以外は引き直すので、セルはどこかが note on になる"
    );
}

/// `randomize_all` と違って音色切替の `stop_live_all()` が後ろに続かないので、
/// この note off を送らないと音が鳴りっぱなしになる。
#[test]
fn keeping_patches_still_silences_sounding_notes() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.rows[0].note = 64;
    state.rows[0].cells[0] = true;
    state.start(now);
    state.poll_steps(now, Duration::ZERO);

    let silenced = state.randomize_keeping_patches(now);

    assert_eq!(messages_of(&silenced), vec![[0x80, 64, 0]]);
    assert_eq!(silenced[0].ahead, SCHEDULE_GUARD);
}

fn messages_of(scheduled: &[GridScheduledMessage]) -> Vec<[u8; 3]> {
    scheduled
        .iter()
        .filter(|item| item.message[0] != 0xB0)
        .map(|item| item.message)
        .collect()
}
