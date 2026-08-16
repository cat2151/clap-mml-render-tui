//! 記録が実態に追随しているか、追随できなくなったことを見つけられるか。

use super::*;

fn note_on(pitch: u8) -> [u8; 3] {
    [NOTE_ON, pitch, 127]
}

fn note_off(pitch: u8) -> [u8; 3] {
    [NOTE_OFF, pitch, 0]
}

#[test]
fn a_new_record_is_silent() {
    let sounding = Sounding::default();

    assert!(sounding.is_silent());
    assert!(!sounding.needs_hard_stop());
    assert!(sounding.note_offs().is_empty());
}

#[test]
fn every_note_on_gets_a_note_off() {
    let mut sounding = Sounding::default();

    sounding.record_sent(&[note_on(60), note_on(64), note_on(67)]);

    assert!(!sounding.is_silent());
    assert_eq!(
        sounding.note_offs(),
        vec![note_off(60), note_off(64), note_off(67)]
    );
}

/// velocity 0 の note on は note off と同じ意味。記録もそちらへ寄せる。
#[test]
fn a_zero_velocity_note_on_counts_as_a_note_off() {
    let mut sounding = Sounding::default();

    sounding.record_sent(&[note_on(60), [NOTE_ON, 60, 0]]);

    assert!(sounding.is_silent());
}

/// 同じ note number へ 2 回 note on を出したら、note off 1 回では止まらない。
/// これが状態機械の破れ。見つけたら以後の停止を音源リセットへ格上げする。
#[test]
fn a_duplicated_note_on_is_reported_and_forces_a_hard_stop() {
    let mut sounding = Sounding::default();

    sounding.record_sent(&[note_on(60)]);
    assert!(!sounding.needs_hard_stop());

    sounding.record_sent(&[note_on(60)]);

    assert!(sounding.needs_hard_stop());
    assert!(sounding.describe().contains("60x2"));
}

/// 鳴らしていない音への note off も、記録がずれている証拠。
#[test]
fn a_note_off_without_a_note_on_is_reported() {
    let mut sounding = Sounding::default();

    sounding.record_sent(&[note_off(60)]);

    assert!(sounding.needs_hard_stop());
}

/// timeline の音は note off では止まらない。積んだら停止は音源リセットになる。
#[test]
fn a_timeline_always_needs_a_hard_stop() {
    let mut sounding = Sounding::default();

    sounding.begin_timeline();

    assert!(!sounding.is_silent());
    assert!(sounding.needs_hard_stop());
    assert!(sounding.note_offs().is_empty());
}

/// 音源リセットで止めたなら実態は確実に黙ったので、ずれの疑いも晴れる。
#[test]
fn a_hard_stop_clears_the_suspicion() {
    let mut sounding = Sounding::default();
    sounding.record_sent(&[note_on(60), note_on(60)]);

    sounding.clear(true);

    assert!(sounding.is_silent());
    assert!(!sounding.needs_hard_stop());
}

/// note off で止めただけなら、ずれの疑いは晴れない。次も音源リセットで止める。
#[test]
fn a_soft_stop_keeps_the_suspicion() {
    let mut sounding = Sounding::default();
    sounding.mark_suspect("test");

    sounding.clear(false);

    assert!(sounding.needs_hard_stop());
}
