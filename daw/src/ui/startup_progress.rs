//! 「音が鳴るまで」の中央 overlay（DAW）。
//!
//! 出す条件と、段階を [`cmrt_tui_core::startup_progress`] の共通ウィジェットへ
//! 翻訳する規則だけを持つ。**描き方は共通ウィジェットに任せる**（画面ごとに
//! 見た目が違うと、同じ待ちなのに別物に見える）。
//!
//! 進捗の出どころは 2 つあり、持ち主が違う:
//!
//! - play server の instance 本数 … supervisor が子プロセスの stderr
//!   （`cmrt-server-startup: instances=N/M`）から拾って持つ
//! - 1 小節目のロード本数 … 演奏スレッドが
//!   [`crate::playback::DawPlaybackStartupState`] へ書く
//!
//! どちらも「持っているところから読む」だけにして、UI 側で数え直さない。

use ratatui::{layout::Rect, Frame};

use cmrt_tui_core::startup_progress::{
    draw_startup_progress_overlay, StartupStep, StartupStepState,
};

use crate::{playback::DawPlaybackStartupStage, DawApp};

const PLAY_SERVER_LABEL: &str = "play server 起動";
const FIRST_MEASURE_LABEL: &str = "1小節目の音色ロード";

pub(super) fn draw_startup_progress(app: &DawApp, f: &mut Frame<'_>, area: Rect) {
    let Some(startup) = app.playback.startup.snapshot() else {
        return;
    };
    let server_progress = app
        .playback
        .realtime_play_server
        .as_ref()
        .and_then(|play_server| play_server.startup_progress())
        .map(|progress| (progress.initialized_instances, progress.total_instances));
    let steps = startup_steps(startup.stage, server_progress);
    draw_startup_progress_overlay(f, area, &steps, startup.started.elapsed());
}

/// 段階を overlay の行へ翻訳する。
///
/// `server_progress` は supervisor が持つ `(起動済み instance 数, 総数)`。
/// 子プロセスを spawn する前は `None`（＝まだ 1 本も報告が無い）。
fn startup_steps(
    stage: DawPlaybackStartupStage,
    server_progress: Option<(usize, usize)>,
) -> Vec<StartupStep> {
    match stage {
        DawPlaybackStartupStage::PlayServer {
            first_measure_follows,
        } => {
            let mut steps = vec![StartupStep::new(
                PLAY_SERVER_LABEL,
                StartupStepState::Running(server_progress),
            )];
            // 小節ごとに SMF を投げる `PlayServer` backend はキャッシュを載せない。
            // 起きない段階を並べると「まだ何か待っている」に読めるので出さない。
            if first_measure_follows {
                steps.push(StartupStep::new(
                    FIRST_MEASURE_LABEL,
                    StartupStepState::Waiting,
                ));
            }
            steps
        }
        DawPlaybackStartupStage::FirstMeasure { loaded, total } => vec![
            StartupStep::new(PLAY_SERVER_LABEL, StartupStepState::Done),
            StartupStep::new(
                FIRST_MEASURE_LABEL,
                StartupStepState::Running(Some((loaded, total))),
            ),
        ],
    }
}

#[cfg(test)]
mod tests;
