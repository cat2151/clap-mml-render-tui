//! loop browser 画面と TuiApp（共有再生ランタイム）を接続する glue。
//!
//! 画面ロジック・再生エンジンは `cmrt-loop-browser` crate 側にあり、ここは
//! TuiApp のフィールド（mode / active_screen / cfg / playback.play_state /
//! begin_playback_session など）に触れる薄い接続層だけを app 側に残す。

use std::sync::Arc;

use crate::tui::loop_browser::{LoopGridChange, LoopPlaybackController, LoopPlaybackGrid};
use crate::tui::{PlayState, TuiApp};

impl<'a> TuiApp<'a> {
    pub(in crate::tui) fn begin_loop_browser_startup(&mut self) {
        self.active_screen = crate::screen_switch::PrimaryScreen::LoopBrowser;
        self.loop_browser.state.starting = true;
    }

    pub(in crate::tui) fn complete_loop_browser_startup(&mut self) {
        if !self.loop_browser.state.starting {
            return;
        }
        let session = self.playback_session.begin();
        self.playback_session
            .set_play_state_if_current(session, PlayState::Idle);
        self.loop_browser
            .state
            .reload(&self.cfg.loop_dirs, &self.cfg.loop_categories);
        if !cfg!(test) {
            self.loop_browser.playback = Some(LoopPlaybackController::spawn(
                self.loop_browser.state.playback_grid(),
                self.loop_browser.state.track_volumes_db().to_vec(),
                self.loop_browser.state.solo_tracks().to_vec(),
                Arc::clone(self.playback_session.play_state()),
                Arc::clone(&self.loop_browser.state.stretch_diagnostics),
                Arc::clone(&self.loop_browser.state.playback_position),
            ));
        }
    }

    pub(in crate::tui) fn stop_loop_browser(&mut self) {
        self.loop_browser.state.starting = false;
        if let Some(mut playback) = self.loop_browser.playback.take() {
            playback.stop();
        }
        let session = self.playback_session.begin();
        self.playback_session
            .set_play_state_if_current(session, PlayState::Idle);
    }

    pub(in crate::tui) fn preview_loop_file(&self, path: std::path::PathBuf, trace_id: u64) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.preview(path, trace_id);
        }
    }

    pub(in crate::tui) fn trigger_loop_pad(&self, pad: char, path: std::path::PathBuf) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.trigger(pad, path);
        }
    }

    pub(in crate::tui) fn update_loop_grid(&self, grid: LoopPlaybackGrid, reason: LoopGridChange) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.set_grid(grid, reason);
        }
    }

    /// 演奏を止めずに裏で次のグリッドを準備させる（オートランダムの先読み）。
    pub(in crate::tui) fn preload_loop_grid(
        &self,
        grid: LoopPlaybackGrid,
        token: u64,
        reason: LoopGridChange,
    ) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.preload_grid(grid, token, reason);
        }
    }

    /// オートランダムの状態機械を 1 フレームぶん進める。
    /// 周回数の判定と抽選は画面側にあり、ここは結果を再生スレッドへ流すだけ。
    pub(in crate::tui) fn pump_loop_browser_step(&mut self) {
        if let crate::tui::loop_browser::LoopBrowserAction::GridPreload {
            grid,
            token,
            reason,
        } = self.loop_browser.state.pump_auto_random()
        {
            self.preload_loop_grid(grid, token, reason);
        }
    }

    pub(in crate::tui) fn restart_loop_grid_at(
        &self,
        grid: LoopPlaybackGrid,
        start_measure: usize,
        reason: LoopGridChange,
    ) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.restart_grid_at(grid, start_measure, reason);
        }
    }

    pub(in crate::tui) fn replace_loop_track_layout(
        &self,
        grid: LoopPlaybackGrid,
        start_measure: usize,
        track_volumes_db: Vec<i32>,
        solo_tracks: Vec<bool>,
    ) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.replace_track_layout(grid, start_measure, track_volumes_db, solo_tracks);
        }
    }

    pub(in crate::tui) fn update_loop_track_volume(&self, track: usize, volume_db: i32) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.set_track_volume(track, volume_db);
        }
    }

    pub(in crate::tui) fn update_loop_track_solo(&self, solo_tracks: Vec<bool>) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.set_track_solo(solo_tracks);
        }
    }

    pub(in crate::tui) fn set_loop_playback_paused(&self, paused: bool, start_measure: usize) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.set_paused(paused, start_measure);
        }
        if paused {
            *self.playback_session.play_state().lock().unwrap() = PlayState::Idle;
        }
    }
}
