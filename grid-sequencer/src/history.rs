//! アプリ終了までだけ保持する Grid の周回履歴と、Daily DAW 用 MML への変換。

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{GridSequencerAction, GridSequencerScreen};

mod export;

pub use export::{
    GridDawChordBinding, GridDawChordSource, GridDawChordVoicing, GridDawLane, GridDawTrack,
    GridSongSnapshot,
};

/// Daily DAW形式の履歴previewが現在どこまで進んでいるか。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum GridHistoryPreviewStatus {
    #[default]
    Idle,
    Rendering {
        completed: usize,
        total: usize,
    },
    Playing,
    Finished,
    Error(String),
}

#[derive(Default)]
pub(crate) struct GridHistory {
    entries: Vec<GridSongSnapshot>,
    selected: usize,
    open: bool,
    previewing: bool,
    resume_on_close: bool,
    resume_requested: bool,
    preview_started_at: Option<Instant>,
    preview_status: GridHistoryPreviewStatus,
}

impl GridHistory {
    fn push(&mut self, snapshot: GridSongSnapshot) {
        if self.entries.last() == Some(&snapshot) {
            return;
        }
        if self.open && !self.entries.is_empty() {
            self.selected += 1;
        }
        self.entries.push(snapshot);
    }

    pub(crate) fn open(&mut self, resume_on_close: bool) {
        self.open = true;
        self.selected = 0;
        self.previewing = false;
        self.resume_on_close = resume_on_close;
        self.resume_requested = false;
        self.preview_started_at = None;
        self.preview_status = GridHistoryPreviewStatus::Idle;
    }

    fn close(&mut self) {
        self.open = false;
        self.previewing = false;
        self.resume_on_close = false;
        self.resume_requested = false;
        self.preview_started_at = None;
        self.preview_status = GridHistoryPreviewStatus::Idle;
    }

    pub(crate) fn selected(&self) -> Option<&GridSongSnapshot> {
        self.entries
            .len()
            .checked_sub(self.selected + 1)
            .and_then(|index| self.entries.get(index))
    }

    fn next_older(&mut self) -> bool {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
            true
        } else {
            false
        }
    }

    fn next_newer(&mut self) -> bool {
        if self.selected > 0 {
            self.selected -= 1;
            true
        } else {
            false
        }
    }

    fn begin_preview(&mut self, now: Instant) {
        self.previewing = true;
        self.preview_started_at = Some(now);
    }

    fn stop_preview(&mut self) {
        self.previewing = false;
        self.preview_started_at = None;
        self.preview_status = GridHistoryPreviewStatus::Idle;
    }

    fn close_and_stop_preview(&mut self) {
        self.stop_preview();
        self.resume_requested = self.resume_on_close;
        self.resume_on_close = false;
        self.open = false;
    }

    pub(crate) fn take_resume_requested(&mut self) -> bool {
        std::mem::take(&mut self.resume_requested)
    }
}

impl GridSequencerScreen {
    /// Historyを閉じたあと、現在のGridを先頭から再開する。
    pub(crate) fn resume_grid_playback(
        &mut self,
        now: Instant,
        ctx: &crate::GridSequencerContext<'_>,
    ) {
        self.cancel_cycle_swap();
        self.state.start_at_bpm(now, self.bpm());
        self.refresh_context(ctx);
        if !self.waiting_for_patches {
            self.prepare_connection_or_start_server(ctx);
        }
    }

    pub(crate) fn absorb_history_snapshots(&mut self) {
        for snapshot in self.state.take_history_snapshots() {
            self.history.push(snapshot);
        }
    }

    pub fn history_open(&self) -> bool {
        self.history.open
    }

    pub(crate) fn close_history(&mut self) {
        self.history.close();
    }

    pub fn set_history_preview_status(&mut self, status: GridHistoryPreviewStatus) {
        if self.history.open {
            self.history.preview_status = status;
        }
    }

    pub(crate) fn history_preview_status(&self) -> &GridHistoryPreviewStatus {
        &self.history.preview_status
    }

    pub(crate) fn history_previewing(&self) -> bool {
        self.history.previewing
    }

    pub(crate) fn history_render_progress(
        &self,
        now: Instant,
    ) -> Option<(usize, usize, std::time::Duration)> {
        let GridHistoryPreviewStatus::Rendering { completed, total } = &self.history.preview_status
        else {
            return None;
        };
        let elapsed = self
            .history
            .preview_started_at
            .map_or(std::time::Duration::ZERO, |started| {
                now.saturating_duration_since(started)
            });
        Some((*completed, *total, elapsed))
    }

    pub(crate) fn history_selected(&self) -> usize {
        self.history.selected
    }

    pub(crate) fn history_rows(&self) -> Vec<String> {
        self.history
            .entries
            .iter()
            .enumerate()
            .rev()
            .map(|(index, snapshot)| {
                let chord = snapshot
                    .chord_label()
                    .map_or_else(|| "chord:OFF".to_string(), |label| format!("chord:{label}"));
                format!(
                    "#{:04}  BPM {:>3.0}  {} meas  {} tracks  {chord}",
                    index + 1,
                    snapshot.bpm(),
                    snapshot.measure_count(),
                    snapshot.track_count(),
                )
            })
            .collect()
    }

    pub(crate) fn handle_history_key(
        &mut self,
        key: KeyEvent,
        now: Instant,
    ) -> GridSequencerAction {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('q'))
            || (key.modifiers.contains(KeyModifiers::SHIFT) && key.code == KeyCode::Char('H'))
        {
            self.history.close_and_stop_preview();
            return GridSequencerAction::StopDailyDawPreview;
        }
        match key.code {
            KeyCode::Char('j') | KeyCode::Down if key.modifiers.is_empty() => {
                if self.history.next_older() {
                    return self.selected_history_preview_action(now);
                }
            }
            KeyCode::Char('k') | KeyCode::Up if key.modifiers.is_empty() => {
                if self.history.next_newer() {
                    return self.selected_history_preview_action(now);
                }
            }
            KeyCode::Char(' ') if key.modifiers.is_empty() => {
                if self.history.previewing {
                    if matches!(
                        self.history.preview_status,
                        GridHistoryPreviewStatus::Rendering { .. }
                            | GridHistoryPreviewStatus::Playing
                    ) {
                        self.history.stop_preview();
                        return GridSequencerAction::StopDailyDawPreview;
                    }
                    return self.selected_history_preview_action(now);
                }
                return self.selected_history_preview_action(now);
            }
            KeyCode::Enter => {
                if let Some(snapshot) = self.history.selected().cloned() {
                    self.close_history();
                    return GridSequencerAction::ImportToDailyDaw(snapshot);
                }
            }
            _ => {}
        }
        GridSequencerAction::Continue
    }

    pub(crate) fn selected_history_preview_action(&mut self, now: Instant) -> GridSequencerAction {
        let Some(snapshot) = self.history.selected().cloned() else {
            return GridSequencerAction::Continue;
        };
        self.history.begin_preview(now);
        self.history.preview_status = GridHistoryPreviewStatus::Rendering {
            completed: 0,
            total: snapshot.track_count(),
        };
        GridSequencerAction::PlayDailyDawPreview(snapshot)
    }
}

#[cfg(test)]
mod tests;
