//! 「音が鳴るまで」の待ちを中央 overlay で見せる（`TuiApp` がホストする画面に共通）。
//!
//! 待ちの大半は server exe の spawn そのものではなく、その後に子プロセスが行う
//! catalog 解決・CLAP 読み込み・instance 生成である。それぞれを別の段階として見せ、
//! 「server 起動」の 1 行へ数秒を押し込めない。
//!
//! chord chart 専用にせず `TuiApp` 共通にしたのは、待ちの実体が共有の
//! [`cmrt_mml_overlay::MmlOverlaySender`] にあり、出す条件も画面ではなく sender の状態
//! （`is_loading()`）で決まるため。同じ sender を通る経路は何もしなくてもこの overlay を得る。
//! MML オーバーレイが開いているときだけは出さない。あちらは自前の `Now loading...`
//! （`mml-overlay/src/ui/loading.rs`）を持っていて、二重に出ると重なる。
//!
//! 描画は DAW / keyboard と同じ [`cmrt_tui_core::startup_progress`]。ここは段階への翻訳だけを持つ。

use std::time::Instant;

use ratatui::Frame;

use cmrt_tui_core::startup_progress::{
    draw_startup_progress_overlay, StartupStep, StartupStepState,
};

use crate::{
    realtime_play::{RealtimePlayServerStartupPhase, RealtimePlayServerStartupProgress},
    screen_switch::PrimaryScreen,
};

use super::TuiApp;

const SERVER_EXE_STEP: &str = "server exe 起動";
const PLUGIN_CATALOG_STEP: &str = "音源カタログの確認";
const LOAD_ENTRY_STEP: &str = "CLAP 音源の読み込み";
const INSTANCES_STEP: &str = "音源 instance 生成";
const AUDIO_STREAM_STEP: &str = "音声出力・待受";

/// 2 段目。既定音色で鳴らすので実際に待つのは SHM の接続だが、名指しできないので
/// まとめて「音源の準備」と呼ぶ。
const SOUND_PREPARE_STEP: &str = "音源の準備";

/// 「音が鳴るまで」の待ち 1 回ぶんの写し。
///
/// server 側の写しは stderr の `cmrt-server-startup:` と localhost の接続確認を
/// supervisor がまとめたもの。描画側は process やファイルを直接調べない。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::tui) struct SoundStartupWait {
    /// 待ち始めた時刻。待っている間は作り直さない（作り直すと経過秒数が 0.0s のままになる）。
    pub(in crate::tui) started_at: Instant,
    /// play server の起動段階。spawn がまだ始まっていなければ `None`。
    pub(in crate::tui) server_startup: Option<RealtimePlayServerStartupProgress>,
}

/// 次のフレームの待ちの写し。出る条件と消える条件はここ 1 か所で決まる。
///
/// 出るのは共有 sender が準備中（`is_loading()`）のあいだだけ。消えるのは準備が終わった瞬間で、
/// 成功か失敗かを問わない（失敗しても `loading` は必ず下りる）。失敗の理由を出すのは
/// [`TuiApp::report_sound_prepare_failure`]。
pub(in crate::tui) fn next_wait(
    previous: Option<SoundStartupWait>,
    loading: bool,
    server_startup: Option<RealtimePlayServerStartupProgress>,
    now: Instant,
) -> Option<SoundStartupWait> {
    if !loading {
        return None;
    }
    Some(SoundStartupWait {
        started_at: previous.map_or(now, |wait| wait.started_at),
        server_startup,
    })
}

/// 待ちの写しを、共通ウィジェットの段階へ翻訳する。
fn startup_steps(wait: &SoundStartupWait) -> Vec<StartupStep> {
    let progress = wait.server_startup;
    let exe_spawned = progress.is_some_and(|progress| progress.server_exe_spawned);
    let server_listening = progress.is_some_and(|progress| progress.server_listening);
    let phase = progress
        .and_then(|progress| progress.phase)
        .or_else(|| exe_spawned.then_some(RealtimePlayServerStartupPhase::PluginCatalog));
    let instances =
        progress.map(|progress| (progress.initialized_instances, progress.total_instances));

    vec![
        StartupStep::new(
            SERVER_EXE_STEP,
            if exe_spawned {
                StartupStepState::Done
            } else {
                StartupStepState::Running(None)
            },
        ),
        StartupStep::new(
            PLUGIN_CATALOG_STEP,
            phase_state(phase, RealtimePlayServerStartupPhase::PluginCatalog, None),
        ),
        StartupStep::new(
            LOAD_ENTRY_STEP,
            phase_state(phase, RealtimePlayServerStartupPhase::LoadEntry, None),
        ),
        StartupStep::new(
            INSTANCES_STEP,
            phase_state(phase, RealtimePlayServerStartupPhase::Instances, instances),
        ),
        StartupStep::new(
            AUDIO_STREAM_STEP,
            audio_stream_state(phase, server_listening),
        ),
        StartupStep::new(
            SOUND_PREPARE_STEP,
            if server_listening {
                StartupStepState::Running(None)
            } else {
                StartupStepState::Waiting
            },
        ),
    ]
}

fn phase_state(
    current: Option<RealtimePlayServerStartupPhase>,
    target: RealtimePlayServerStartupPhase,
    count: Option<(usize, usize)>,
) -> StartupStepState {
    match current {
        Some(current) if current > target => StartupStepState::Done,
        Some(current) if current == target => StartupStepState::Running(count),
        _ => StartupStepState::Waiting,
    }
}

/// audio stream と listen は短く、利用者からはどちらも「server が音を出せるまで」の
/// 最終段階なので 1 行へまとめる。port の接続確認が取れるまでは完了にしない。
fn audio_stream_state(
    phase: Option<RealtimePlayServerStartupPhase>,
    server_listening: bool,
) -> StartupStepState {
    if server_listening {
        StartupStepState::Done
    } else if phase.is_some_and(|phase| phase >= RealtimePlayServerStartupPhase::AudioStream) {
        StartupStepState::Running(None)
    } else {
        StartupStepState::Waiting
    }
}

/// overlay が出た／消えた瞬間のログ 1 行。変わっていなければ `None`。
/// `global_log_sink` はテストでは no-op なので、組み立てだけを名前のある関数へ出してある。
/// 実機で「overlay が何秒出ていたか」を残す唯一の証跡。
fn wait_transition_log_line(
    previous: Option<SoundStartupWait>,
    next: Option<SoundStartupWait>,
    now: Instant,
) -> Option<String> {
    match (previous, next) {
        (None, Some(_)) => Some("sound-startup: event=wait-begin".to_string()),
        (Some(wait), None) => Some(format!(
            "sound-startup: event=wait-end elapsed_ms={}",
            now.saturating_duration_since(wait.started_at).as_millis()
        )),
        _ => None,
    }
}

/// 中央 overlay を描く。
pub(super) fn draw(f: &mut Frame<'_>, wait: &SoundStartupWait, now: Instant) {
    let steps = startup_steps(wait);
    let elapsed = now.saturating_duration_since(wait.started_at);
    draw_startup_progress_overlay(f, f.area(), &steps, elapsed);
}

impl TuiApp<'_> {
    /// 待ちの写しを作り直す。呼ぶのは `ui::draw` の冒頭 1 か所だけ。
    ///
    /// ランタイムのループではなく描画の入口に置いたのは、配線ごと描画テストで確かめるため
    /// （ループの中に置くと、この呼び出しを消してもどのテストも落ちない）。
    /// 読むのは mutex 越しの値 2 つだけで、待つ処理は通さない（`running_server_generation` は
    /// TCP connect で最大 150ms 塞ぐので呼ばない）。
    pub(in crate::tui) fn sync_sound_startup_wait(&mut self, now: Instant) {
        // sender が無いのはテストのときだけ。描画テストが写しを直に置けるよう、触らない。
        let Some(sender) = &self.mml_overlay_sender else {
            return;
        };
        let status = sender.status();
        let server_startup = self.play_server.startup_progress();
        let previous = self.sound_startup_wait;
        self.sound_startup_wait = next_wait(previous, status.is_loading(), server_startup, now);
        if let Some(line) = wait_transition_log_line(previous, self.sound_startup_wait, now) {
            crate::logging::global_log_sink(&line);
        }
        let prepare_error = status.prepare_error().map(str::to_string);
        self.report_sound_prepare_failure(prepare_error);
    }

    /// 音源の準備が失敗した理由を画面へ出す。overlay は準備が終わった瞬間に消えるので、
    /// これが無いと「overlay が黙って消えて、音も鳴らない」になる。
    ///
    /// 出せる欄を持っているのは chord chart だけ（下段 1 行）。他の画面は `play_server_notice` に任せる。
    /// 同じ理由は 1 度しか出さない。理由は次の command が来ても持ち越される
    /// （`mml-overlay` の `begin_status`）ので、毎フレーム上書きするとユーザーが消した下段がすぐ戻る。
    pub(in crate::tui) fn report_sound_prepare_failure(&mut self, error: Option<String>) {
        if self.reported_sound_prepare_error == error {
            return;
        }
        self.reported_sound_prepare_error = error.clone();
        let Some(error) = error else {
            return;
        };
        crate::logging::global_log_sink(&format!(
            "sound-startup: event=prepare-failed error=\"{error}\""
        ));
        if self.active_screen == PrimaryScreen::ChordChart {
            self.chord_chart.error = Some(format!("鳴らせません: {error}"));
        }
    }
}

#[cfg(test)]
mod tests;
