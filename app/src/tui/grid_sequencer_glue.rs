//! grid sequencer 画面と TuiApp（共有ランタイム）を接続する glue。
//!
//! 画面ロジック・ステップ進行・MIDI 送信は `crate::tui::grid_sequencer` 側にあり、
//! ここは TuiApp のフィールド（active_screen / cfg / patch_load_state / voicing /
//! chord_catalog / playback）に触れる薄い接続層だけを残す。
//!
//! ctx は必ずフィールドから直接組み立てること。`&mut self` を取るヘルパー越しに
//! 作ると、ctx が借りている voicing / chord_catalog と画面の `&mut` が衝突する。

use std::time::Instant;

use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;

use cmrt_chord::ChordProgressionCatalog;

use crate::realtime_play::PatchVoicing;
use crate::tui::grid_sequencer::{
    GridConnectionStatus, GridPatchLoad, GridSequencerAction, GridSequencerContext,
    GridVoicingLookup,
};
use crate::tui::voicing::VoicingState;
use crate::tui::{PatchLoadState, PlayState, PrimaryScreen, TuiApp};

impl GridVoicingLookup for VoicingState {
    fn cached_voicing(&self, patch: &str) -> Option<PatchVoicing> {
        self.layers.resolve(&self.cache, patch)
    }
}

struct GridContextParts<'ctx> {
    patch_dirs_configured: bool,
    patch_load: &'ctx PatchLoadState,
    voicing: &'ctx VoicingState,
    chord_catalog: &'ctx ChordProgressionCatalog,
    chord_patch_categories: &'ctx [String],
    chord_source_updated: bool,
}

fn grid_sequencer_context<'ctx>(parts: GridContextParts<'ctx>) -> GridSequencerContext<'ctx> {
    GridSequencerContext {
        patch_dirs_configured: parts.patch_dirs_configured,
        patch_load: match parts.patch_load {
            PatchLoadState::Loading => GridPatchLoad::Loading,
            PatchLoadState::Ready(pairs) => GridPatchLoad::Ready(pairs),
            PatchLoadState::Err(error) => GridPatchLoad::Err(error),
        },
        chord_catalog: parts.chord_catalog,
        voicing: parts.voicing,
        chord_patch_categories: parts.chord_patch_categories,
        chord_source_updated: parts.chord_source_updated,
    }
}

impl TuiApp<'_> {
    /// 画面へ入る。初回はランダムな grid を作り、いずれも即座に再生を始める。
    pub(in crate::tui) fn enter_grid_sequencer(&mut self) {
        let session = self.playback_session.begin();
        self.playback_session
            .set_play_state_if_current(session, PlayState::Idle);

        // コード進行カタログはここで読む（キャッシュがまだ無い初回だけ待たされる）。
        self.chord_catalog = self.chord_progression_source.catalog();

        let patch_dirs_configured = crate::patches::has_configured_patch_dirs(&self.cfg);
        let patch_load = self.patch_load_state.lock().unwrap();
        let ctx = grid_sequencer_context(GridContextParts {
            patch_dirs_configured,
            patch_load: &patch_load,
            voicing: &self.voicing,
            chord_catalog: &self.chord_catalog,
            chord_patch_categories: &self.cfg.chord_patch_categories,
            chord_source_updated: false,
        });
        self.grid_sequencer.enter(Instant::now(), &ctx);
        drop(patch_load);

        self.active_screen = PrimaryScreen::GridSequencer;
    }

    /// 前回 grid sequencer 画面で終了していた場合、起動直後に再生を始める。
    /// `switch_to_primary_screen` を通らない経路なので、run() の冒頭で一度だけ呼ぶ。
    pub(in crate::tui) fn enter_restored_grid_sequencer(&mut self) {
        if self.active_screen == PrimaryScreen::GridSequencer {
            self.enter_grid_sequencer();
        }
    }

    pub(in crate::tui) fn handle_grid_sequencer_key_event(
        &mut self,
        key: KeyEvent,
    ) -> GridSequencerAction {
        let patch_dirs_configured = crate::patches::has_configured_patch_dirs(&self.cfg);
        // patch 一覧の MutexGuard は画面へ渡す間だけ保持する。
        let patch_load = self.patch_load_state.lock().unwrap();
        let ctx = grid_sequencer_context(GridContextParts {
            patch_dirs_configured,
            patch_load: &patch_load,
            voicing: &self.voicing,
            chord_catalog: &self.chord_catalog,
            chord_patch_categories: &self.cfg.chord_patch_categories,
            chord_source_updated: false,
        });
        self.grid_sequencer.handle_key(key, Instant::now(), &ctx)
    }

    pub(in crate::tui) fn handle_grid_sequencer_mouse_event(
        &mut self,
        mouse: MouseEvent,
        terminal_area: Rect,
    ) {
        let patch_dirs_configured = crate::patches::has_configured_patch_dirs(&self.cfg);
        let patch_load = self.patch_load_state.lock().unwrap();
        let ctx = grid_sequencer_context(GridContextParts {
            patch_dirs_configured,
            patch_load: &patch_load,
            voicing: &self.voicing,
            chord_catalog: &self.chord_catalog,
            chord_patch_categories: &self.cfg.chord_patch_categories,
            chord_source_updated: false,
        });
        self.grid_sequencer.handle_mouse(mouse, terminal_area, &ctx);
    }

    /// 1フレームぶん進める。コード進行データの更新アナウンスを出し終えたら true を
    /// 返し、共有ランタイムがアプリを再起動する。
    pub(in crate::tui) fn pump_grid_sequencer_step(&mut self) -> bool {
        // 更新通知は一度しか取れないので、画面へ渡すのはこの1回だけ。
        let chord_source_updated = self.chord_progression_source.take_update_notice();
        let patch_dirs_configured = crate::patches::has_configured_patch_dirs(&self.cfg);
        let patch_load = self.patch_load_state.lock().unwrap();
        let ctx = grid_sequencer_context(GridContextParts {
            patch_dirs_configured,
            patch_load: &patch_load,
            voicing: &self.voicing,
            chord_catalog: &self.chord_catalog,
            chord_patch_categories: &self.cfg.chord_patch_categories,
            chord_source_updated,
        });
        self.grid_sequencer.refresh_context(&ctx);
        self.grid_sequencer.pump_step(Instant::now(), &ctx)
    }

    pub(in crate::tui) fn finish_grid_sequencer(&mut self) {
        self.grid_sequencer.finish();
    }

    pub(in crate::tui) fn grid_sequencer_connection_status(&self) -> GridConnectionStatus {
        self.grid_sequencer.connection_status()
    }
}
