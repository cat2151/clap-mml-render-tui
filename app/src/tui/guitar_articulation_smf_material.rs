//! Guitar Articulation 画面の SMF 素材の I/O。
//!
//! 画面は file を読まない。画面が求めたパス（相対ならプロセスの cwd から）をここで読み、
//! note だけを ch1 にまとめたイベント列にして画面へ渡す。

use std::path::Path;

use cmrt_guitar_articulation::TimedMidiEvent;

use crate::tui::guitar_articulation::GuitarArticulationAction;
use crate::tui::TuiApp;

/// SMF を読み、note on/off だけを ch1 にまとめた時刻つきイベント列にする。
fn read_smf_notes(path: &Path) -> Result<Vec<TimedMidiEvent>, String> {
    std::fs::read(path)
        .map_err(|error| error.to_string())
        .and_then(|bytes| cmrt_chord::timed_smf_notes(&bytes).map(|timed| timed.events))
}

impl TuiApp<'_> {
    /// `path` を読んで画面の素材にし、画面が返した action（読めたら Articulated の演奏）を返す。
    pub(in crate::tui) fn load_guitar_articulation_smf(
        &mut self,
        path: &Path,
    ) -> GuitarArticulationAction {
        let events = read_smf_notes(path);
        self.guitar_articulation
            .load_smf(path.to_path_buf(), events)
    }
}
