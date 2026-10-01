//! 自動スライドアウト（[`Rule::AutoSlideOut`]）: 列の奏法を変えずに、離すときの CC24 だけ選ぶ。

use super::*;

#[test]
fn auto_slide_out_sends_88_then_back_to_13() {
    let out = events_for("o3 l8 e g a", &rules_on(&[(1, Rule::AutoSlideOut)], &[]));
    assert_eq!(
        values_of(&out, 24),
        vec![(0.25, AUTO_SLIDE_OUT_VALUE), (0.5, 13), (0.75, 13)]
    );
    // E6（88）は release6 の音域（G#1〜E6）の内側。ポジション移動（30〜76）と違って効く。
    assert!(!events_for("o7 l8 e e", &rules_on(&[(1, Rule::AutoSlideOut)], &[])).is_empty());
}

#[test]
fn auto_slide_out_keeps_the_columns_articulation() {
    let notes = notes_from_events(&cmrt_chord::timed_performance("o3 l8 e g").unwrap().events);
    let rules = rules_on(&[(1, Rule::HammerPull), (1, Rule::AutoSlideOut)], &[]);
    assert!(rules.is_on(1, Rule::HammerPull) && rules.is_on(1, Rule::AutoSlideOut));
    assert_eq!(
        articulate(&notes, &rules)[1].articulation,
        Articulation::HammerOn
    );
    assert_eq!(
        values_of(&events_for("o3 l8 e g", &rules), 24),
        vec![(0.25, AUTO_SLIDE_OUT_VALUE), (0.5, 13), (0.5, 13)]
    );
}

#[test]
fn auto_slide_out_does_not_reach_mute_down() {
    // Mute_Down の離し音は、CC24 が 80 以上だとスライドアウトではなくアップストロークになる。
    let muted = rules_on(&[(1, Rule::PalmMute), (1, Rule::AutoSlideOut)], &[]);
    assert!(events_for("o3 l8 e g a", &muted).is_empty());
}

#[test]
fn auto_slide_out_and_position_release_take_turns_in_a_column() {
    let mut rules = rules_on(
        &[(1, Rule::PositionRelease), (2, Rule::PositionRelease)],
        &[],
    );
    rules.toggle(1, Rule::AutoSlideOut);
    assert!(rules.is_on(1, Rule::AutoSlideOut));
    assert!(!rules.is_on(1, Rule::PositionRelease));
    assert!(rules.is_on(2, Rule::PositionRelease));
    rules.toggle(1, Rule::PositionRelease);
    assert!(!rules.is_on(1, Rule::AutoSlideOut));
    assert!(!Rule::AutoSlideOut.is_exclusive());
}

#[test]
fn neighbouring_columns_each_get_their_own_cc24() {
    // 列 1 が自動スライドアウト、列 2 がポジション移動。列 1 の戻しは列 2 の値より前に積む。
    let rules = rules_on(&[(1, Rule::AutoSlideOut), (2, Rule::PositionRelease)], &[]);
    assert_eq!(
        values_of(&events_for("o3 l8 e g a", &rules), 24),
        vec![
            (0.25, AUTO_SLIDE_OUT_VALUE),
            (0.5, 13),
            (0.5, POSITION_RELEASE_VALUE),
            (0.75, 13),
            (0.75, 13),
        ]
    );
}

#[test]
fn auto_slide_out_wins_over_humanize_release_in_its_column() {
    let raw = cmrt_chord::timed_performance("o3 l8 e g a e")
        .unwrap()
        .events;
    let release = [crate::RowRule::HumanizeRelease];
    let with = crate::convert(&raw, &rules_on(&[(1, Rule::AutoSlideOut)], &release));
    let at = |seconds: f64| -> Vec<u8> {
        values_of(&with, 24)
            .into_iter()
            .filter(|(at, _)| *at == seconds)
            .map(|(_, value)| value)
            .collect()
    };
    assert_eq!(at(0.25), vec![AUTO_SLIDE_OUT_VALUE]);
}

/// 演奏音（KS を除く）の note off ごとの、その時点の CC24（並べた順に効かせる）。
fn cc24_at_note_offs(events: &[TimedMidiEvent]) -> Vec<(u8, u8)> {
    let mut cc24 = 13;
    let mut out = Vec::new();
    for e in events {
        match e.message[0] & 0xF0 {
            0xB0 if e.message[1] == 24 => cc24 = e.message[2],
            0x80 if e.message[1] >= 28 => out.push((e.message[1], cc24)),
            0x90 if e.message[1] >= 28 && e.message[2] == 0 => out.push((e.message[1], cc24)),
            _ => {}
        }
    }
    out
}

#[test]
fn only_the_auto_slide_out_column_releases_with_88_even_with_humanize() {
    // 汚しで前の音の off が最後の c2 の on を越えると、前の音が c2 の CC24 = 88 で離れてスライドアウトする行。
    let raw = cmrt_chord::timed_performance("t130l24 c2 r cdefgfedcdefgfedc2")
        .unwrap()
        .events;
    for rows in [
        &[crate::RowRule::EconomyPicking, crate::RowRule::Humanize][..],
        &[
            crate::RowRule::EconomyPicking,
            crate::RowRule::Humanize,
            crate::RowRule::HumanizeRelease,
        ],
    ] {
        let rules = rules_on(&[(0, Rule::PickScratch), (17, Rule::AutoSlideOut)], rows);
        let offs = cc24_at_note_offs(&crate::convert(&raw, &rules));
        let (last, before) = offs.split_last().unwrap();
        assert_eq!(*last, (60, AUTO_SLIDE_OUT_VALUE), "{rows:?}");
        assert!(
            before.iter().all(|&(_, cc24)| cc24 != AUTO_SLIDE_OUT_VALUE),
            "{rows:?}: {offs:?}"
        );
    }
}
