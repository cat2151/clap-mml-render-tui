//! grid sequencer 画面と TuiApp（共有ランタイム）を接続する glue。
//!
//! 画面ロジック・ステップ進行・MIDI 送信は `crate::tui::grid_sequencer` 側にあり、
//! ここは TuiApp のフィールド（active_screen / cfg / patch_load_state / playback）に
//! 触れる薄い接続層だけを残す。

use std::time::Instant;

use crossterm::event::KeyEvent;

use crate::tui::grid_sequencer::{
    GridConnectionStatus, GridPatchLoad, GridSequencerAction, GridSequencerContext,
};
use crate::tui::{PatchLoadState, PlayState, PrimaryScreen, TuiApp};

fn grid_sequencer_context<'ctx>(
    patch_dirs_configured: bool,
    patch_load: &'ctx PatchLoadState,
) -> GridSequencerContext<'ctx> {
    GridSequencerContext {
        patch_dirs_configured,
        patch_load: match patch_load {
            PatchLoadState::Loading => GridPatchLoad::Loading,
            PatchLoadState::Ready(pairs) => GridPatchLoad::Ready(pairs),
            PatchLoadState::Err(error) => GridPatchLoad::Err(error),
        },
    }
}

impl TuiApp<'_> {
    /// 画面へ入る。初回はランダムな grid を作り、いずれも即座に再生を始める。
    pub(in crate::tui) fn enter_grid_sequencer(&mut self) {
        let session = self.playback_session.begin();
        self.playback_session
            .set_play_state_if_current(session, PlayState::Idle);

        let patch_dirs_configured = crate::patches::has_configured_patch_dirs(&self.cfg);
        let patch_load = self.patch_load_state.lock().unwrap();
        let ctx = grid_sequencer_context(patch_dirs_configured, &patch_load);
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
        let ctx = grid_sequencer_context(patch_dirs_configured, &patch_load);
        self.grid_sequencer.handle_key(key, Instant::now(), &ctx)
    }

    pub(in crate::tui) fn pump_grid_sequencer_step(&mut self) {
        let patch_dirs_configured = crate::patches::has_configured_patch_dirs(&self.cfg);
        let patch_load = self.patch_load_state.lock().unwrap();
        let ctx = grid_sequencer_context(patch_dirs_configured, &patch_load);
        self.grid_sequencer.refresh_context(&ctx);
        drop(patch_load);
        self.grid_sequencer.pump_step(Instant::now());
    }

    pub(in crate::tui) fn finish_grid_sequencer(&mut self) {
        self.grid_sequencer.finish();
    }

    pub(in crate::tui) fn grid_sequencer_connection_status(&self) -> GridConnectionStatus {
        self.grid_sequencer.connection_status()
    }
}
