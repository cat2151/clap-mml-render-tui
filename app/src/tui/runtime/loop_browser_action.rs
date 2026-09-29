//! Loop Browser のキーが返した action を、音の操作へ写す。

use crate::tui::loop_browser::LoopBrowserAction;
use crate::tui::TuiApp;

impl TuiApp<'_> {
    /// action を実行する。`true` はアプリ終了（ループを抜ける）。
    pub(super) fn apply_loop_browser_action(&mut self, action: LoopBrowserAction) -> bool {
        match action {
            LoopBrowserAction::Continue => {}
            LoopBrowserAction::Preview(path) => {
                let trace_id = self
                    .loop_browser
                    .state
                    .take_preview_trace()
                    .unwrap_or_else(crate::tui::loop_browser::performance::next_trace_id);
                self.preview_loop_file(path, trace_id);
            }
            LoopBrowserAction::Trigger { pad, path } => self.trigger_loop_pad(pad, path),
            LoopBrowserAction::GridReplaced {
                start_measure,
                grid,
                reason,
            } => {
                self.restart_loop_grid_at(grid, start_measure, reason);
            }
            LoopBrowserAction::GridRefresh { grid, reason } => self.update_loop_grid(grid, reason),
            LoopBrowserAction::GridPreload {
                grid,
                token,
                mode,
                reason,
            } => self.preload_loop_grid(grid, token, mode, reason),
            LoopBrowserAction::TrackLayoutChanged {
                start_measure,
                grid,
                track_volumes_db,
                solo_tracks,
            } => self.replace_loop_track_layout(grid, start_measure, track_volumes_db, solo_tracks),
            LoopBrowserAction::TrackVolumeChanged { track, volume_db } => {
                self.update_loop_track_volume(track, volume_db)
            }
            LoopBrowserAction::TrackSoloChanged { solo_tracks } => {
                self.update_loop_track_solo(solo_tracks)
            }
            LoopBrowserAction::SetPlaybackPaused {
                paused,
                start_measure,
            } => self.set_loop_playback_paused(paused, start_measure),
            LoopBrowserAction::BpmChanged { mode, grid } => self.set_loop_bpm_mode(mode, grid),
            LoopBrowserAction::Quit => {
                self.stop_loop_browser();
                return true;
            }
        }
        false
    }
}
