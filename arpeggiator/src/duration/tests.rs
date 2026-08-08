use super::clamp_durations;

#[test]
fn same_voice_returning_within_the_desired_length_truncates_it() {
    // 声部2つを交互に鳴らすと、同じ音は2 step後に来る。4 stepの希望は2で打ち切られる。
    // 末尾の2音は同じ声部が戻ってこないので、代わりに小節末で打ち切られる。
    let voices = [0, 1, 0, 1];
    assert_eq!(clamp_durations(&voices, &[4, 4, 4, 4]), [2, 2, 2, 1]);
}

#[test]
fn a_distant_return_leaves_the_desired_length_alone() {
    // 4声部のup arpeggioなら同じ声部は4 step後。4 stepの希望はそのまま通る。
    let voices = [0, 1, 2, 3, 0, 1, 2, 3];
    assert_eq!(
        clamp_durations(&voices, &[4, 4, 4, 4, 1, 1, 1, 1]),
        [4, 4, 4, 4, 1, 1, 1, 1]
    );
}

#[test]
fn the_end_of_the_bar_clamps_the_tail() {
    let voices = [0, 1, 2];
    assert_eq!(clamp_durations(&voices, &[4, 4, 4]), [3, 2, 1]);
}

#[test]
fn durations_never_drop_below_one_step() {
    let voices = [0, 0, 0, 0];
    assert_eq!(clamp_durations(&voices, &[4, 2, 4, 2]), [1, 1, 1, 1]);
    assert_eq!(clamp_durations(&voices, &[0, 0, 0, 0]), [1, 1, 1, 1]);
}

#[test]
fn a_missing_desired_length_falls_back_to_one_step() {
    let voices = [0, 1, 2, 3];
    assert_eq!(clamp_durations(&voices, &[4]), [4, 1, 1, 1]);
    assert_eq!(clamp_durations(&voices, &[]), [1, 1, 1, 1]);
}

#[test]
fn an_empty_series_produces_nothing() {
    assert_eq!(clamp_durations(&[], &[4]), Vec::<usize>::new());
}

#[test]
fn notes_never_reach_the_next_attack_of_the_same_voice() {
    let voices = [0, 3, 1, 3, 2, 3, 0, 3, 1, 3, 2, 3, 0, 3, 1, 3];
    let durations = clamp_durations(&voices, &[4; 16]);
    for (step, duration) in durations.iter().enumerate() {
        let end = step + duration;
        assert!(
            !voices[step + 1..end].contains(&voices[step]),
            "step {step} (voice {}) overlapped its own next attack",
            voices[step]
        );
        assert!(end <= voices.len(), "step {step} ran past the bar");
    }
}
