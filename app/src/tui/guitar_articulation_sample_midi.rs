//! Guitar Articulation 画面のサンプル MID モードの I/O。
//!
//! 画面は file を読まない。一覧（置き場の `*.mid`）と読み込み（SMF → 時刻つきイベント列）をここで行い、
//! 結果を画面へ渡す。置き場は画面の音色と同じ規則で、Sforzando の置き場から解決する。

use std::path::{Path, PathBuf};

use cmrt_effect_chain_select::chain_json;
use cmrt_tui_core::patch_load::PatchLoadState;
use cmrt_tui_core::patch_plugins::PatchPlugins;

use crate::guitar_articulation_events::play_sample_midi_log_line;
use crate::tui::guitar_articulation::{PATCH, SAMPLE_MIDI_DIR};
use crate::tui::TuiApp;

/// `dir` 直下の `*.mid`（拡張子の大文字小文字を問わない）を名前順で返す。
/// `dir` を読めなければ、パスを入れた理由を返す。
pub(crate) fn sample_midi_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(dir).map_err(|error| {
        format!(
            "サンプル MID の置き場を読めません: {} ({error})",
            dir.display()
        )
    })?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("mid"))
        })
        .collect();
    files.sort();
    Ok(files)
}

/// 音色の一覧から、サンプル MID の置き場を解決する。
fn sample_midi_dir(state: &PatchLoadState) -> Result<PathBuf, String> {
    match state {
        PatchLoadState::Loading => Err("音色の一覧を読み込み中です".to_string()),
        PatchLoadState::Err(error) => Err(format!("音色の一覧を読めません: {error}")),
        PatchLoadState::Ready(snapshot) => sample_midi_dir_in(snapshot.patch_plugins()),
    }
}

/// 画面の音色（[`PATCH`]）を鳴らすプラグインの置き場の規則で、[`SAMPLE_MIDI_DIR`] を実パスへ直す。
pub(crate) fn sample_midi_dir_in(plugins: &PatchPlugins) -> Result<PathBuf, String> {
    let plugin = plugins
        .for_patch(PATCH)
        .map_err(|error| format!("{PATCH} の置き場が分かりません: {error}"))?;
    Ok(PathBuf::from(plugin.base.resolve(SAMPLE_MIDI_DIR)))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

impl TuiApp<'_> {
    /// 置き場の `*.mid` を画面の一覧 overlay へ渡す。
    pub(in crate::tui) fn open_guitar_articulation_sample_midi_list(&mut self) {
        let dir = sample_midi_dir(&self.patch_load_state.lock().unwrap());
        let files = dir.and_then(|dir| sample_midi_files(&dir));
        self.guitar_articulation.open_sample_midi_list(files);
    }

    /// 一覧で選んだ file を読み、CC と pitch bend も残したイベント列を画面へ渡す。
    pub(in crate::tui) fn load_guitar_articulation_sample_midi(&mut self, path: &Path) {
        let events = std::fs::read(path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| cmrt_chord::timed_smf_events(&bytes).map(|timed| timed.events));
        self.guitar_articulation
            .load_sample_midi(file_name(path), events);
    }

    /// MID の全体（`None`）か 1 音（`Some(i)`）を、MML の演奏と同じ音色・chain で鳴らす。
    pub(in crate::tui) fn play_guitar_articulation_sample_midi(&mut self, note: Option<usize>) {
        let Some(file) = self
            .guitar_articulation
            .sample_midi()
            .map(|midi| midi.name().to_string())
        else {
            return;
        };
        let events = self.guitar_articulation.sample_midi_events(note);
        let effect_chain = chain_json(self.guitar_articulation.sounding_effect_chain());
        let patch = self
            .sync_guitar_articulation_instrument(&effect_chain)
            .patch();
        crate::logging::global_log_sink(&play_sample_midi_log_line(&file, note, &events, patch));
        self.play_guitar_articulation_events(events, &effect_chain, patch);
        self.preload_full_guitar(&effect_chain);
    }
}
