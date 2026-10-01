//! サンプル MID モード。付属の MID を一覧から選んで読み、右 pane に全イベントを出して鳴らす。
//!
//! file の一覧と読み込みは host が行い（[`GuitarArticulationAction::OpenSampleMidiList`] /
//! [`GuitarArticulationAction::LoadSampleMidi`]）、画面は結果を受け取るだけ。
//! `h` / `l` は 1 音モードに依らず移動先の 1 音を求める。
//! MID モードの間も MML・ルール・カーソルはそのまま残し、`Esc` で抜けるとそのまま戻る。

use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent};

use crate::{SampleMidi, TimedMidiEvent};

use super::{GuitarArticulationAction, GuitarArticulationScreen};

/// 開いている MID と、カーソルのある音のまとまり。
pub(super) struct SampleMidiState {
    midi: SampleMidi,
    cursor: usize,
}

/// 開いている一覧 overlay。
pub(super) struct SampleMidiList {
    files: Vec<PathBuf>,
    selected: usize,
}

impl GuitarArticulationScreen {
    /// 一覧 overlay を開く。MID を開いていれば、その file を選んだ状態で開く。
    /// 読めなかったか 1 つも無ければ、開かずに理由を `error` に出す。
    pub fn open_sample_midi_list(&mut self, files: Result<Vec<PathBuf>, String>) {
        let files = match files {
            Ok(files) if files.is_empty() => {
                self.error = Some("サンプル MID が見つかりません".to_string());
                return;
            }
            Ok(files) => files,
            Err(reason) => {
                self.error = Some(reason);
                return;
            }
        };
        let current = self.sample_midi.as_ref().map(|state| state.midi.name());
        let selected = current
            .and_then(|name| files.iter().position(|path| file_name(path) == name))
            .unwrap_or(0);
        self.sample_midi_list = Some(SampleMidiList { files, selected });
    }

    /// 一覧で選んだ file の読み込み結果を受ける。読めたら一覧を閉じて MID モードへ入る（鳴らさない）。
    /// 読めなければ一覧を開いたまま理由を `error` に出す。
    pub fn load_sample_midi(&mut self, name: String, events: Result<Vec<TimedMidiEvent>, String>) {
        match events {
            Ok(events) => {
                self.sample_midi = Some(SampleMidiState {
                    midi: SampleMidi::new(name, events),
                    cursor: 0,
                });
                self.sample_midi_list = None;
                self.error = None;
            }
            Err(reason) => self.error = Some(format!("{name}: {reason}")),
        }
    }

    /// 開いている MID。MID モードでなければ `None`。
    pub fn sample_midi(&self) -> Option<&SampleMidi> {
        self.sample_midi.as_ref().map(|state| &state.midi)
    }

    /// MID モードでカーソルのある音のまとまり。
    pub fn sample_midi_cursor(&self) -> usize {
        self.sample_midi.as_ref().map_or(0, |state| state.cursor)
    }

    /// 鳴らすイベント列。`None` は MID の全体、`Some(i)` はまとまり `i` の 1 音。MID が無ければ空。
    pub fn sample_midi_events(&self, note: Option<usize>) -> Vec<TimedMidiEvent> {
        let Some(midi) = self.sample_midi() else {
            return Vec::new();
        };
        match note {
            None => midi.events().to_vec(),
            Some(group) => midi.note_events(group),
        }
    }

    /// 一覧 overlay の file と選んでいる位置。閉じていれば `None`。
    pub fn sample_midi_list(&self) -> Option<(&[PathBuf], usize)> {
        self.sample_midi_list
            .as_ref()
            .map(|list| (list.files.as_slice(), list.selected))
    }

    /// 一覧 overlay を開いている間のキー。ここで全部受け、画面のキーへは渡さない。
    /// `j` / `k` は選び直すたびに、その file の試聴（[`GuitarArticulationAction::PreviewSampleMidi`]）を求める。
    pub(super) fn handle_sample_midi_list_key(
        &mut self,
        key: KeyEvent,
    ) -> GuitarArticulationAction {
        let Some(list) = self.sample_midi_list.as_mut() else {
            return GuitarArticulationAction::Continue;
        };
        match key.code {
            KeyCode::Esc => {
                self.sample_midi_list = None;
                self.error = None;
            }
            KeyCode::Enter => {
                return GuitarArticulationAction::LoadSampleMidi(list.files[list.selected].clone());
            }
            KeyCode::Char('j') | KeyCode::Down => {
                list.selected = (list.selected + 1).min(list.files.len() - 1);
                return GuitarArticulationAction::PreviewSampleMidi(
                    list.files[list.selected].clone(),
                );
            }
            KeyCode::Char('k') | KeyCode::Up => {
                list.selected = list.selected.saturating_sub(1);
                return GuitarArticulationAction::PreviewSampleMidi(
                    list.files[list.selected].clone(),
                );
            }
            _ => {}
        }
        GuitarArticulationAction::Continue
    }

    /// MID モードのキー。MML 側のキー（ルール・`b`・`x`・`i`・履歴など）は効かない。
    pub(super) fn handle_sample_midi_key(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        let note_preview = self.note_preview;
        let Some(state) = self.sample_midi.as_mut() else {
            return GuitarArticulationAction::Continue;
        };
        let moved = match key.code {
            KeyCode::Char('h') | KeyCode::Left => state.cursor.saturating_sub(1),
            KeyCode::Char('l') | KeyCode::Right => {
                (state.cursor + 1).min(state.midi.group_count().saturating_sub(1))
            }
            KeyCode::Char(' ') => {
                return match (note_preview, state.midi.group_count()) {
                    (false, _) => GuitarArticulationAction::PlaySampleMidi { note: None },
                    (true, 0) => GuitarArticulationAction::Continue,
                    (true, _) => GuitarArticulationAction::PlaySampleMidi {
                        note: Some(state.cursor),
                    },
                };
            }
            KeyCode::Char('n') => {
                self.note_preview = !note_preview;
                return GuitarArticulationAction::Continue;
            }
            KeyCode::Char('o') => return GuitarArticulationAction::OpenSampleMidiList,
            KeyCode::Esc => {
                self.sample_midi = None;
                return GuitarArticulationAction::Continue;
            }
            KeyCode::Char('q') => return GuitarArticulationAction::Quit,
            _ => return GuitarArticulationAction::Continue,
        };
        // `h` / `l` は 1 音モードに依らず、移動先（端なら今）のまとまりを鳴らす。
        if state.midi.group_count() == 0 {
            return GuitarArticulationAction::Continue;
        }
        state.cursor = moved;
        GuitarArticulationAction::PlaySampleMidi { note: Some(moved) }
    }
}

fn file_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
