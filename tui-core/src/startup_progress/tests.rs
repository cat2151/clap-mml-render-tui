use super::*;

fn rendered(steps: &[StartupStep], elapsed: Duration) -> String {
    startup_progress_lines(steps, elapsed)
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_running_step_shows_its_count_and_a_partially_filled_bar() {
    let text = rendered(
        &[StartupStep::new(
            "play server 起動",
            StartupStepState::Running(Some((7, 14))),
        )],
        Duration::from_millis(1_740),
    );

    assert!(text.contains("play server 起動"), "{text}");
    assert!(text.contains("7/14"), "{text}");
    assert!(text.contains(&"█".repeat(BAR_WIDTH / 2)), "{text}");
    assert!(text.contains("経過 1.7s"), "{text}");
}

/// 総数が分からない段階でも「動いている」ことは出す。0/0 と書くと
/// 「1 件も無い」に読めてしまうため、件数は `…` にする。
#[test]
fn a_running_step_without_counts_shows_that_it_started() {
    let text = rendered(
        &[StartupStep::new(
            "play server 起動",
            StartupStepState::Running(None),
        )],
        Duration::ZERO,
    );

    assert!(text.contains('▶'), "{text}");
    assert!(text.contains('…'), "{text}");
    assert!(!text.contains('█'), "{text}");
}

#[test]
fn a_waiting_step_is_distinguishable_from_a_finished_one() {
    let text = rendered(
        &[
            StartupStep::new("先", StartupStepState::Done),
            StartupStep::new("後", StartupStepState::Waiting),
        ],
        Duration::ZERO,
    );

    assert!(text.contains("✓ 先"), "{text}");
    assert!(text.contains("done"), "{text}");
    assert!(text.contains("後"), "{text}");
    assert!(text.contains('-'), "{text}");
}

/// 枠の幅が毎フレーム変わると読めない。どの状態でもバーの桁数は同じにする。
#[test]
fn every_step_state_draws_the_same_bar_width() {
    for state in [
        StartupStepState::Waiting,
        StartupStepState::Running(None),
        StartupStepState::Running(Some((3, 14))),
        StartupStepState::Running(Some((0, 0))),
        StartupStepState::Done,
    ] {
        assert_eq!(
            Span::raw(progress_bar(state)).width(),
            BAR_WIDTH,
            "{state:?}"
        );
    }
}

/// 完了数が総数を超えても、バーは伸び切りで止まる（`repeat` が panic しない）。
#[test]
fn a_count_beyond_the_total_does_not_overflow_the_bar() {
    assert_eq!(
        Span::raw(progress_bar(StartupStepState::Running(Some((99, 14))))).width(),
        BAR_WIDTH
    );
}
