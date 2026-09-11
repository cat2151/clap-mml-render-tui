//! 「音が鳴るまで」の待ちを中央 overlay で見せる（`TuiApp` がホストする画面に共通）。
//!
//! ## なぜ要るか
//!
//! chord chart 画面（`Ctrl+G` → `C`）の preview は、**最初の 1 回だけ数秒無音のまま
//! 待たされ、その間画面には何も出ていなかった**。押したのに反応が無いのか壊れたのかが
//! 分からない。待ちの実体は音色ロードではなく play server プロセスの起動で、その中でも
//! CLAP instance の生成が支配的（実測で warm 1.7 秒、cold 4.9 秒）。音色ロードは
//! ほぼ 0ms（preview は既定音色 = `patch: None` で鳴らすため。
//! `chord_chart_glue::PREVIEW_PATCH` を参照）。
//!
//! ## どこに置いたか
//!
//! **chord chart 専用にせず、`TuiApp` 共通の overlay にした。** 待ちの実体は
//! chord chart ではなく共有の [`cmrt_mml_overlay::MmlOverlaySender`] にあり、
//! この sender は `Ctrl+P` の MML オーバーレイと chord chart の preview が共有している。
//! 出す条件も画面ではなく sender の状態で決まる（`is_loading()`）ので、
//! 同じ sender を通る経路は何もしなくてもこの overlay を得る。
//! 置き場所も `play_server_notice`（「音が鳴らない理由」を出す既存の共通 overlay）と
//! 同じ `ui::draw` の末尾にそろえてある。
//!
//! MML オーバーレイが開いているときだけは出さない。あちらは自前の
//! `Now loading...`（`mml-overlay/src/ui/loading.rs`）を持っていて、二重に出ると重なる。
//! **あちらを作り直さない**ための線引きで、他画面の既存 overlay には手を入れていない。
//!
//! ## 描き方
//!
//! DAW / keyboard と同じ共通ウィジェット（[`cmrt_tui_core::startup_progress`]）。
//! 中身に合わせた枠を `centered_text_block_rect` で中央へ置くところまで
//! あちらが持っているので、ここは**段階への翻訳だけ**を持つ。

use std::time::Instant;

use ratatui::Frame;

use cmrt_tui_core::startup_progress::{
    draw_startup_progress_overlay, StartupStep, StartupStepState,
};

use crate::screen_switch::PrimaryScreen;

use super::TuiApp;

/// 1 段目。play server プロセスが listen するまで。
///
/// keyboard 画面の overlay と同じ綴りにしてある（同じ待ちなので）。
const PLAY_SERVER_STEP: &str = "play server 起動";

/// 2 段目。SHM 接続と音色ロード。
///
/// keyboard 側は「音色ロード」だが、こちらは既定音色（`patch: None`）で鳴らすため
/// 実際に待つのは SHM の接続。名指しできない以上、まとめて「音源の準備」と呼ぶ。
const SOUND_PREPARE_STEP: &str = "音源の準備";

/// 「音が鳴るまで」の待ち 1 回ぶんの写し。
///
/// **持つのは実際に知り得ることだけ。** 段階を細かく見せたくても、アプリが
/// 知っているのは「sender が準備中か」と「play server の instance が何本できたか」の
/// 2 つしかない（instance 数は play server の stderr が
/// `cmrt-server-startup: instances=N/M` で流してくるものを supervisor が拾っている）。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::tui) struct SoundStartupWait {
    /// 待ち始めた時刻。**待っている間は作り直さない**
    /// （毎フレーム作り直すと経過秒数がずっと 0.0s のままになる）。
    pub(in crate::tui) started_at: Instant,
    /// play server の instance 生成の進み具合。**spawn がまだ始まっていなければ `None`**。
    pub(in crate::tui) server_startup: Option<(usize, usize)>,
}

/// 次のフレームの待ちの写し。**出る条件と消える条件はここ 1 か所で決まる。**
///
/// - 出る: 共有 sender が準備中（`is_loading()`）のあいだだけ。
/// - 消える: 準備が終わった瞬間。**成功したか失敗したかを問わない**
///   （失敗しても `loading` は必ず下りるので、出っぱなしになりようがない）。
///
/// 失敗して消えたときに理由を出すのは
/// [`TuiApp::report_sound_prepare_failure`] の仕事で、ここではない。
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

/// 待ちの写しを、共通ウィジェットの段階へ翻訳する。
///
/// **知り得ない段階をでっち上げない。** 出せるのは 2 段だけで、
/// 1 段目が終わったと言えるのは instance が全部できたと play server 自身が
/// 言ってきたときだけ。
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
/// **`None`（spawn がまだ始まっていない）は「まだ」に倒す。** 実測で、押してから
/// spawn までに 0.3〜1.6 秒（実体の解決と探索）掛かっている。ここを「済んだ」に
/// 倒すと、その 1.6 秒のあいだ画面が嘘をつく。
/// 逆に「すでに listen 済みのサーバーへ相乗りした」ときは 1 段目が短いあいだ
/// 回りっぱなしに見えるが、そちらは 2 段目ごとすぐ終わる。
fn server_instances_ready(progress: Option<(usize, usize)>) -> bool {
    matches!(progress, Some((done, total)) if total > 0 && done >= total)
}

/// overlay が出た／消えた瞬間のログ 1 行。変わっていなければ `None`。
///
/// **`global_log_sink` はテストでは no-op** なので、組み立てだけを名前のある関数へ
/// 出しておく（`chord_chart_glue::preview_request_log_line` と同じ手）。
/// これが実機で「overlay が何秒出ていたか」を残す唯一の証跡になる。
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
    /// 待ちの写しを作り直す。**呼ぶのは `ui::draw` の冒頭 1 か所だけ。**
    ///
    /// ランタイムのループではなく描画の入口に置いたのは、**配線ごと描画テストで
    /// 確かめられるようにする**ため（ループの中に置くと、この呼び出しを消しても
    /// どのテストも落ちない）。sender が無いテストでは何もしないので、
    /// 描画テストは写しを直に置いて overlay の出方を見られる。
    ///
    /// 読むのは mutex 越しの値 2 つだけで、**接続確認のような待つ処理は一切通らない**
    /// （`running_server_generation` は TCP connect で最大 150ms 塞ぐので呼ばない）。
    /// 実際の待ちは sender の worker スレッドが引き受けているので、
    /// この関数が呼ばれるループはキー入力も再描画も止まらない。
    pub(in crate::tui) fn sync_sound_startup_wait(&mut self, now: Instant) {
        // sender が無いのはテストのときだけ。写しには触らないでおく
        // （描画テストが写しを置いて overlay を見られるようにする）。
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

    /// 音源の準備が失敗した理由を画面へ出す。
    ///
    /// overlay は準備が終わった瞬間に消えるので、これが無いと
    /// 「overlay が黙って消えて、音も鳴らない」になる。
    ///
    /// **出せる欄を持っているのは chord chart だけ**（下段 1 行）。他の画面は
    /// 従来どおり `play_server_notice`（play server が起動できないことの知らせ）に
    /// 任せる。他画面に無かった欄をここで足すと、画面の作り直しになる。
    ///
    /// 同じ理由は 1 度しか出さない。理由は次の command が来ても持ち越されるので
    /// （`mml-overlay` の `begin_status`）、毎フレーム上書きすると
    /// ユーザーが次のキーで消した下段がすぐ戻ってしまう。
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
