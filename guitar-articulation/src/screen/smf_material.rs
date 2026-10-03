//! SMF 素材。任意の SMF の note を MML の代わりの素材にし、raw・Articulated・matrix を作る。
//!
//! file は host が読み（[`GuitarArticulationAction::LoadSmf`]）、画面は結果を
//! [`GuitarArticulationScreen::load_smf`] で受け取るだけ。素材の間は MML 欄の MML を残したまま
//! `plain` と `notes` だけを SMF から作るので、ルール・`h` / `l`・`b` / `space`・1 音モードはそのまま効く。
//! 素材は MML 文字列で表せないので、履歴へは積まず、アルペジエーターと history overlay は開かない。
//! `i` で MML を確定すると素材を捨てて MML 素材へ戻る。

use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui_textarea::TextArea;

use crate::{notes_from_events, TimedMidiEvent};

use super::input::is_commit_key;
use super::{history, is_shift_key, GuitarArticulationAction, GuitarArticulationScreen, Take};

/// 読み込み overlay を開くキー。
const LOAD_KEY: char = 'O';
/// 単音化を切り替えるキー。
const TOP_NOTE_KEY: char = 'M';

/// SMF 素材まわりの状態。素材が無くても、最後に読んだパスと単音化の on/off はセッションの間残す。
#[derive(Default)]
pub(super) struct SmfState {
    /// 読み込み overlay を開いている間だけ `Some`。
    input: Option<TextArea<'static>>,
    /// SMF を素材にしている間だけ `Some`。
    material: Option<SmfMaterial>,
    /// このセッションで最後に読めたパス。overlay の初期値。
    last_path: Option<String>,
    /// 単音化（[`cmrt_midi_filter::keep_top_notes`]）を当てるか。
    top_note: bool,
}

/// 素材にしている SMF。
struct SmfMaterial {
    name: String,
    path: PathBuf,
    /// 読んだままの（単音化する前の）イベント列。
    events: Vec<TimedMidiEvent>,
}

impl SmfState {
    pub(super) fn is_material(&self) -> bool {
        self.material.is_some()
    }

    pub(super) fn input_open(&self) -> bool {
        self.input.is_some()
    }

    /// MML 素材へ戻る。overlay・最後のパス・単音化の on/off は残す。
    pub(super) fn discard_material(&mut self) {
        self.material = None;
    }
}

impl GuitarArticulationScreen {
    /// SMF を素材にしている間だけ、その file 名。
    pub fn smf_material_name(&self) -> Option<&str> {
        self.smf
            .material
            .as_ref()
            .map(|material| material.name.as_str())
    }

    /// SMF を素材にしている間だけ、読んだパス。
    pub fn smf_material_path(&self) -> Option<&Path> {
        self.smf
            .material
            .as_ref()
            .map(|material| material.path.as_path())
    }

    /// 単音化が on か。SMF 素材でない間も値は保つ。
    pub fn smf_top_note(&self) -> bool {
        self.smf.top_note
    }

    /// 読み込み overlay を開いているか。
    pub fn smf_input_open(&self) -> bool {
        self.smf.input_open()
    }

    /// 読み込み overlay の入力欄。閉じていれば `None`。
    pub(crate) fn smf_input(&self) -> Option<&TextArea<'static>> {
        self.smf.input.as_ref()
    }

    /// 読み込み結果を受ける。読めたら overlay を閉じて素材にし、列ルールを消して Articulated を鳴らす。
    /// 読めなければ overlay を開いたまま `"<file 名>: <理由>"` を `error` に出す。
    pub fn load_smf(
        &mut self,
        path: PathBuf,
        events: Result<Vec<TimedMidiEvent>, String>,
    ) -> GuitarArticulationAction {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let events = match events {
            Ok(events) => events,
            Err(reason) => {
                self.error = Some(format!("{name}: {reason}"));
                return GuitarArticulationAction::Continue;
            }
        };
        self.smf.input = None;
        self.smf.last_path = Some(path.display().to_string());
        self.smf.material = Some(SmfMaterial { name, path, events });
        self.error = None;
        self.rebuild_smf_material()
    }

    /// SMF 素材から raw と音を作り直す。列の数と並びが変わるので列ルールは消し、行全体のルールは残す。
    fn rebuild_smf_material(&mut self) -> GuitarArticulationAction {
        let Some(material) = self.smf.material.as_ref() else {
            return GuitarArticulationAction::Continue;
        };
        let events = if self.smf.top_note {
            cmrt_midi_filter::keep_top_notes(&material.events)
        } else {
            material.events.clone()
        };
        self.material_from_chord = false;
        self.notes = notes_from_events(&events);
        self.plain = events;
        self.rules = self.rules.without_column_rules();
        self.cursor = 0;
        self.rebuild_converted();
        GuitarArticulationAction::Play(Take::Converted)
    }

    /// SMF 素材のキー（`O`・`M`）と、素材の間は開かない overlay のキー（`z`・`Shift+H`）を受ける。
    /// それ以外は `None` を返し、画面のキーへ回す。
    pub(super) fn handle_smf_key(&mut self, key: KeyEvent) -> Option<GuitarArticulationAction> {
        if is_shift_key(key, LOAD_KEY) {
            let initial = self.smf.last_path.as_deref().unwrap_or_default();
            self.smf.input = Some(cmrt_tui_core::text_input::new_single_line_textarea(initial));
            return Some(GuitarArticulationAction::Continue);
        }
        if is_shift_key(key, TOP_NOTE_KEY) {
            if !self.smf.is_material() {
                self.error = Some("O で SMF を読んでください".to_string());
                return Some(GuitarArticulationAction::Continue);
            }
            self.smf.top_note = !self.smf.top_note;
            return Some(self.rebuild_smf_material());
        }
        if !self.smf.is_material() {
            return None;
        }
        if key.code == KeyCode::Char('z') && key.modifiers.is_empty() {
            self.error = Some(
                "SMF 素材の間はアルペジエーターを開けません（i で MML を確定すると戻ります）"
                    .to_string(),
            );
            return Some(GuitarArticulationAction::Continue);
        }
        if history::is_history_key(key) {
            self.error =
                Some("SMF 素材の間は履歴を開けません（i で MML を確定すると戻ります）".to_string());
            return Some(GuitarArticulationAction::Continue);
        }
        None
    }

    /// 読み込み overlay を開いている間のキー。ここで全部受け、画面のキーへは渡さない。
    pub(super) fn handle_smf_input_key(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        let Some(input) = self.smf.input.as_mut() else {
            return GuitarArticulationAction::Continue;
        };
        if key.code == KeyCode::Esc {
            self.smf.input = None;
            self.error = None;
            return GuitarArticulationAction::Continue;
        }
        if is_commit_key(key) {
            let value = cmrt_tui_core::text_input::textarea_value(input);
            let path = trim_path(&value);
            if path.is_empty() {
                self.error = Some("SMF のパスを入力してください".to_string());
                return GuitarArticulationAction::Continue;
            }
            return GuitarArticulationAction::LoadSmf(PathBuf::from(path));
        }
        if cmrt_tui_core::text_input::apply_key_event_to_textarea(input, key) {
            self.error = None;
        }
        GuitarArticulationAction::Continue
    }
}

/// 前後の空白と `"` を削る（エクスプローラーの「パスのコピー」は `"` で囲む）。
fn trim_path(value: &str) -> &str {
    value.trim_matches(|ch: char| ch.is_whitespace() || ch == '"')
}

#[cfg(test)]
mod tests;
