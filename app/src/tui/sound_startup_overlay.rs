//! 「音が鳴るまで」の待ちを中央 overlay で見せる（`TuiApp` がホストする画面に共通）。
//!
//! 待ちの実体は音色ロードではなく play server プロセスの起動で、CLAP instance の生成が
//! 支配的（数秒）。音色ロードはほぼ 0ms（preview は既定音色 `patch: None` で鳴らす。
//! `chord_chart_glue::PREVIEW_PATCH`）。
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

use crate::screen_switch::PrimaryScreen;

use super::TuiApp;

/// 1 段目。play server プロセスが listen するまで（keyboard 画面の overlay と同じ綴り）。
const PLAY_SERVER_STEP: &str = "play server 起動";

/// 2 段目。既定音色で鳴らすので実際に待つのは SHM の接続だが、名指しできないので
/// まとめて「音源の準備」と呼ぶ。
const SOUND_PREPARE_STEP: &str = "音源の準備";

/// 「音が鳴るまで」の待ち 1 回ぶんの写し。
///
/// 持つのは実際に知り得ることだけ。アプリが知っているのは「sender が準備中か」と
/// 「play server の instance が何本できたか」（stderr の `cmrt-server-startup: instances=N/M`
/// を supervisor が拾う）の 2 つしかない。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::tui) struct SoundStartupWait {
    /// 待ち始めた時刻。待っている間は作り直さない（作り直すと経過秒数が 0.0s のままになる）。
    pub(in crate::tui) started_at: Instant,
    /// play server の instance 生成の進み具合。spawn がまだ始まっていなければ `None`。
    pub(in crate::tui) server_startup: Option<(usize, usize)>,
}

/// 次のフレームの待ちの写し。出る条件と消える条件はここ 1 か所で決まる。
///
/// 出るのは共有 sender が準備中（`is_loading()`）のあいだだけ。消えるのは準備が終わった瞬間で、
/// 成功か失敗かを問わない（失敗しても `loading` は必ず下りる）。失敗の理由を出すのは
/// [`TuiApp::report_sound_prepare_failure`]。
pub(in crate::tui) fn next_wait(
    previous: Option<SoundStartupWait>,
    loading: bool,
    server_startup: Option<(usize, usize)>,
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

/// 待ちの写しを、共通ウィジェットの段階へ翻訳する。知り得ない段階をでっち上げない:
/// 1 段目が終わったと言えるのは、instance が全部できたと play server 自身が言ってきたときだけ。
fn startup_steps(wait: &SoundStartupWait) -> Vec<StartupStep> {
    if server_instances_ready(wait.server_startup) {
        vec![
            StartupStep::new(PLAY_SERVER_STEP, StartupStepState::Done),
            // SHM 接続も音色ロードも件数を持たないので、数えられるものが無い。
            StartupStep::new(SOUND_PREPARE_STEP, StartupStepState::Running(None)),
        ]
    } else {
        vec![
            StartupStep::new(
                PLAY_SERVER_STEP,
                StartupStepState::Running(wait.server_startup),
            ),
            StartupStep::new(SOUND_PREPARE_STEP, StartupStepState::Waiting),
        ]
    }
}

/// play server の instance が全部そろったか。
///
/// `None`（spawn がまだ始まっていない）は「まだ」に倒す。押してから spawn までに
/// 1 秒超（実体の解決と探索）掛かることがあり、「済んだ」に倒すとそのあいだ画面が嘘をつく。
/// listen 済みのサーバーへ相乗りしたときは 1 段目が短く回り続けて見えるが、2 段目ごとすぐ終わる。
fn server_instances_ready(progress: Option<(usize, usize)>) -> bool {
    matches!(progress, Some((done, total)) if total > 0 && done >= total)
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
        let server_startup = self
            .play_server
            .startup_progress()
            .map(|progress| (progress.initialized_instances, progress.total_instances));
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
