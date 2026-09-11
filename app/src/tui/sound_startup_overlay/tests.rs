//! 「音が鳴るまで」の overlay の、出る条件・消える条件・段階の翻訳。
//!
//! 実際に描かれるかどうかは `ui/tests/sound_startup_overlay.rs`（描画された buffer を
//! 読む）が見る。ここは判断だけを見る。

use std::time::{Duration, Instant};

use cmrt_tui_core::startup_progress::StartupStepState;

use super::{
    next_wait, startup_steps, wait_transition_log_line, SoundStartupWait, PLAY_SERVER_STEP,
    SOUND_PREPARE_STEP,
};
use crate::tui::ui::tests::{render_lines, test_config};
use crate::tui::TuiApp;

fn wait_at(now: Instant, server_startup: Option<(usize, usize)>) -> SoundStartupWait {
    SoundStartupWait {
        started_at: now,
        server_startup,
    }
}

/// 出る条件: 共有 sender が準備中のあいだ。
#[test]
fn the_wait_appears_while_the_shared_sender_is_preparing() {
    let now = Instant::now();

    let wait = next_wait(None, true, Some((3, 14)), now).expect("準備中なら待ちが立つこと");

    assert_eq!(wait.started_at, now);
    assert_eq!(wait.server_startup, Some((3, 14)));
}

/// 待っているあいだ、開始時刻は作り直さない（経過秒数が 0.0s に戻らない）。
#[test]
fn the_elapsed_clock_keeps_running_across_frames() {
    let started = Instant::now();
    let previous = wait_at(started, None);
    let later = started + Duration::from_secs(3);

    let wait = next_wait(Some(previous), true, Some((5, 14)), later).expect("まだ準備中");

    assert_eq!(
        wait.started_at, started,
        "待ち始めた時刻は持ち越すこと（毎フレーム作り直すと経過がずっと 0.0s になる）"
    );
    assert_eq!(wait.server_startup, Some((5, 14)), "進み具合は更新すること");
}

/// 消える条件: 準備が終わったら消える。
#[test]
fn the_wait_disappears_once_the_preparation_finishes() {
    let now = Instant::now();
    let previous = wait_at(now, Some((14, 14)));

    assert_eq!(next_wait(Some(previous), false, Some((14, 14)), now), None);
}

/// 失敗したときも同じ経路で消える。**出っぱなしにならない。**
///
/// 失敗は `loading` が下りることでしか伝わってこないので、成功と同じ 1 本の経路になる。
#[test]
fn a_failed_preparation_closes_the_wait_too() {
    let now = Instant::now();
    let previous = wait_at(now, None);

    assert_eq!(
        next_wait(Some(previous), false, None, now),
        None,
        "失敗しても loading は必ず下りる。出っぱなしにする経路を作らないこと"
    );
}

/// spawn がまだ始まっていない（進み具合が無い）あいだは、1 段目を「済んだ」にしない。
#[test]
fn the_play_server_step_is_not_done_before_the_spawn_reports_anything() {
    let steps = startup_steps(&wait_at(Instant::now(), None));

    assert_eq!(steps[0].label, PLAY_SERVER_STEP);
    assert_eq!(steps[0].state, StartupStepState::Running(None));
    assert_eq!(steps[1].label, SOUND_PREPARE_STEP);
    assert_eq!(steps[1].state, StartupStepState::Waiting);
}

/// instance の本数はそのまま 1 段目の進み具合になる。
#[test]
fn the_play_server_step_shows_the_instance_count() {
    let steps = startup_steps(&wait_at(Instant::now(), Some((5, 14))));

    assert_eq!(steps[0].state, StartupStepState::Running(Some((5, 14))));
    assert_eq!(steps[1].state, StartupStepState::Waiting);
}

/// instance が全部そろったら 2 段目へ進む。
#[test]
fn the_second_step_starts_once_every_instance_is_ready() {
    let steps = startup_steps(&wait_at(Instant::now(), Some((14, 14))));

    assert_eq!(steps[0].state, StartupStepState::Done);
    assert_eq!(steps[1].state, StartupStepState::Running(None));
}

/// 失敗して overlay が消えたとき、理由が chord chart の下段へ出る。
#[test]
fn the_failure_reason_reaches_the_chord_chart_bottom_line() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::ChordChart;

    app.report_sound_prepare_failure(Some("play server が起動できません".to_string()));

    let lines = render_lines(&mut app, 120, 30);
    assert!(
        lines.iter().any(|line| {
            let squeezed: String = line.chars().filter(|c| !c.is_whitespace()).collect();
            squeezed.contains("鳴らせません:playserverが起動できません")
        }),
        "消えた理由を画面へ出すこと: {lines:?}"
    );
}

/// 同じ理由は出し直さない（次のキーで消した下段が、毎フレーム戻ってきてしまう）。
#[test]
fn the_same_failure_is_reported_only_once() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::ChordChart;
    let reason = "boom".to_string();

    app.report_sound_prepare_failure(Some(reason.clone()));
    app.chord_chart.error = None;
    app.report_sound_prepare_failure(Some(reason));

    assert_eq!(app.chord_chart.error, None);
}

/// 実機で「overlay が何秒出ていたか」を残す。出た／消えた瞬間だけ 1 行ずつ。
#[test]
fn the_log_records_when_the_overlay_opened_and_how_long_it_stayed() {
    let started = Instant::now();
    let waiting = Some(wait_at(started, Some((5, 14))));
    let later = started + Duration::from_millis(1699);

    assert_eq!(
        wait_transition_log_line(None, waiting, started).as_deref(),
        Some("sound-startup: event=wait-begin")
    );
    assert_eq!(
        wait_transition_log_line(waiting, None, later).as_deref(),
        Some("sound-startup: event=wait-end elapsed_ms=1699")
    );
    assert_eq!(
        wait_transition_log_line(waiting, waiting, later),
        None,
        "待っている間は毎フレーム書かないこと"
    );
    assert_eq!(wait_transition_log_line(None, None, later), None);
}
