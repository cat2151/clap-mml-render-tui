//! 6 画面（notepad / DAW / keyboard / loop browser / grid sequencer / chord chart）を
//! ホストする共有ランタイム。
//!
//! 各画面の実装本体は画面ごとの crate に閉じている（DAW は `crate::daw`）。ここに置くのは
//! 「どの画面か」「画面をまたいで共有するもの」と、各 crate と接続する `*_glue` だけ。

pub(crate) use cmrt_notepad as notepad;
mod notepad_glue;
pub(crate) use cmrt_keyboard as keyboard;
mod keyboard_glue;
pub(crate) use cmrt_loop_browser as loop_browser;
mod loop_browser_glue;
pub(crate) use cmrt_grid_sequencer as grid_sequencer;
mod grid_sequencer_glue;
// chord chart は音を鳴らさない画面なので、glue はキーの配送と保存だけ。
pub(crate) use cmrt_chord_chart as chord_chart;
mod chord_chart_glue;
// MML 入力オーバーレイ（どの画面からでも Ctrl+P）。glue は開閉のきっかけと MIDI 送信をつなぐだけ。
pub(crate) use cmrt_mml_overlay as mml_overlay;
mod mml_overlay_glue;
// `cmrt patch-roles` 診断。voicing の解決を TUI と同じ経路で行うため、画面ランタイム側に置く。
pub mod patch_role_report;
mod play_server_notice;
mod runtime;
mod session;
mod sound_startup_overlay;
mod ui;
mod voicing;

use ratatui::Frame;

use std::sync::{Arc, Mutex};

use cmrt_chord::ChordProgressionCatalog;
use cmrt_tui_core::playback_session::PlaybackSession;

use crate::chord_progression_source::ChordProgressionSource;

use self::chord_chart::ChordChartScreen;
use self::grid_sequencer::GridSequencerScreen;
use self::keyboard::KeyboardScreen;
use self::loop_browser::LoopBrowserScreen;
use self::mml_overlay::{MmlOverlay, MmlOverlaySender};
use self::notepad::{Mode, NormalAction, NotepadScreen};
use self::voicing::VoicingState;
use crate::config::Config;
use crate::screen_switch::{PrimaryScreen, ScreenSwitchMenu};
pub(crate) use cmrt_tui_core::patch_load::PatchLoadState;
pub(crate) use cmrt_tui_core::PlayState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuiExitReason {
    Quit,
    RestartApp,
}

/// 共有 MML overlay を現在どの機能が借りているか。
///
/// patch の正本を owner ごとに分けるだけでなく、Chord Chart の確定先を行 index ではなく
/// stable id で持つ。overlay が開いている間に section が消えても別の行へ誤って書かない。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) enum MmlOverlayOwner {
    Global,
    ChordChart {
        section_id: chord_chart::SectionId,
    },
    /// Chord Chart から直接開いた、layer 別の音色選択。
    ChordChartPatch {
        role: cmrt_patches::PatchRole,
    },
}

/// 画面ホスト。持つのは「どの画面か」と、画面をまたいで共有するものだけ。
pub struct TuiApp<'a> {
    pub(super) active_screen: PrimaryScreen,
    pub(super) screen_switch_menu: ScreenSwitchMenu,
    cfg: Arc<Config>,
    /// カタログのプラグインごとのロード済み CLAP entry。
    /// render_server backend / テストでは空。
    plugin_entries: cmrt_offline_render::PluginEntries,
    pub(in crate::tui) notepad: NotepadScreen<'a>,
    pub(in crate::tui) keyboard: KeyboardScreen<'a>,
    pub(in crate::tui) loop_browser: LoopBrowserScreen,
    pub(in crate::tui) grid_sequencer: GridSequencerScreen,
    /// コード進行の「構成」画面。preview は MML オーバーレイと同じ経路を借りる。
    pub(in crate::tui) chord_chart: ChordChartScreen,
    /// 直近に Chord Chart の通常 preview として sender へ渡した command。
    /// sender が公開する同じ command の実演奏区間だけを sounding として画面へ書き戻す。
    chord_chart_preview_command_id: Option<u64>,
    /// Bass patch catalog の初期ロードが終わるまで保留している最新の preview。
    ///
    /// Bass ON なのに Chord だけ先に鳴らすと、起動直後の 1 回だけ Bass が無音になる。
    /// loader 完了後に同じ要求を自動で流すため、host 側で 1 件だけ持つ。
    deferred_chord_chart_preview: Option<chord_chart::PreviewRequest>,
    /// Grid履歴をimport前に1小節だけoffline試聴する、揮発性のplayer/cache。
    grid_history_preview: crate::daw::DawGridPreviewPlayer,
    /// どの画面からでも開ける MML 入力オーバーレイ。開くと現在の画面の演奏は止まり、
    /// keyboard 画面と同じ音源インスタンスを借りる。
    pub(in crate::tui) mml_overlay: MmlOverlay<'a>,
    /// 共有 overlay の現在の借り手。閉じているときは `None`。
    pub(in crate::tui) mml_overlay_owner: Option<MmlOverlayOwner>,
    /// 通常の `Ctrl+P` overlay が持つ canonical patch。
    pub(in crate::tui) mml_overlay_patch: Option<String>,
    /// Chord Chart の編集・通常 preview が持つ canonical patch。
    pub(in crate::tui) chord_chart_patch: Option<String>,
    /// Chord Chart の Bass layer が持つ、Chord とは独立した canonical patch。
    pub(in crate::tui) chord_chart_bass_patch: Option<String>,
    /// 送信先。テストでは `None`（音は鳴らさず状態遷移だけ確かめる）。
    mml_overlay_sender: Option<MmlOverlaySender>,
    /// patch ごとの mono/poly 判定結果のキャッシュ。keyboard 画面と
    /// grid sequencer の chord mode（poly patch 抽選）が読む。
    pub(in crate::tui) voicing: VoicingState,
    /// コード進行カタログの取得・キャッシュ。grid sequencer の chord mode 専用。
    pub(in crate::tui) chord_progression_source: ChordProgressionSource,
    /// 読み込み済みのコード進行カタログ。grid sequencer 画面へ入るときに読む
    /// （キャッシュがまだ無い初回だけ待たされるため、起動時には読まない）。
    pub(in crate::tui) chord_catalog: ChordProgressionCatalog,
    /// バックグラウンドスレッドが収集したパッチリストの状態。
    /// notepad（音色選択）と keyboard（patch catalog）が同じ実体を読む。
    patch_load_state: Arc<Mutex<PatchLoadState>>,
    /// 設定不足でカタログから外れたプラグインの案内
    /// （[`cmrt_runtime::SkippedCatalogPlugin::notice_line`]）。音色選択を開く画面へ配る。
    /// 一覧に出てこないものの話なので `patch_load_state` からは分からず、起動時に 1 回だけ数える。
    pub(in crate::tui) catalog_notes: Vec<String>,
    /// 再生セッションの世代管理。notepad・keyboard・loop browser で共有する。
    pub(in crate::tui) playback_session: PlaybackSession,
    /// 各画面が共有する play server の監督。ここが持つのは
    /// 「起動できていないことを画面に出す」ためだけ（送信は各 sender が持つ）。
    play_server: Arc<cmrt_realtime_play::RealtimePlayServerSupervisor>,
    /// ユーザーが閉じた知らせ。同じ理由で出し直さないために覚えておく。
    dismissed_play_server_failure: Option<cmrt_realtime_play::ServerStartupFailure>,
    /// 「音が鳴るまで」の待ちの写し。`None` は待っていない。
    /// 作るのは `sync_sound_startup_wait` で、描画は読むだけ。
    pub(in crate::tui) sound_startup_wait: Option<sound_startup_overlay::SoundStartupWait>,
    /// 画面へ出し終えた「音源の準備に失敗した理由」。同じ理由を出し直さないために覚えておく。
    reported_sound_prepare_error: Option<String>,
}

impl<'a> TuiApp<'a> {
    fn draw(&mut self, f: &mut Frame) {
        ui::draw(self, f);
    }
}

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod tests;
