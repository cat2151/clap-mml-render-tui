//! アルペジエーター overlay（`z`）の素材リスト。MML と chord を区別せず同じリストに持つ。
//! 素材 pane は 1 行 1 素材の入力欄で、param pane の素材の行で鳴らす素材を選ぶ。
//! 切り替えは overlay の素材だけを変え、MML 欄・列ごとのルール・履歴には触らない。

use crossterm::event::{KeyCode, KeyEvent};
use ratatui_textarea::{CursorMove, TextArea};

use super::{GuitarArticulationAction, GuitarArticulationScreen, Take};

impl GuitarArticulationScreen {
    /// 保存済みの overlay の素材を持たせる。空は「まだ選んでいない」。解釈できるかは overlay を開くときに確かめる。
    pub fn with_arp_material(mut self, material: String) -> Self {
        self.arp_material = material;
        self
    }

    /// 設定 file へ書く値。overlay の素材は選んでいなければ空のまま書く。
    pub fn settings(&self) -> crate::GuitarArticulationSettings {
        crate::GuitarArticulationSettings {
            startup_instrument: self.startup_instrument,
            arp_materials: self.arp_materials.clone(),
            arp_material: self.arp_material.clone(),
            arp: self.arp,
        }
    }

    /// 保存済みの素材リストを持たせる。
    pub fn with_arp_materials(mut self, materials: Vec<String>) -> Self {
        self.arp_materials = materials;
        self
    }

    /// overlay で選ぶ素材の並び。
    pub fn arp_materials(&self) -> &[String] {
        &self.arp_materials
    }

    /// overlay の素材が素材リストの何番目か。リストに無ければ `None`。
    pub fn arp_material_position(&self) -> Option<usize> {
        self.arp_materials
            .iter()
            .position(|material| material == self.arp_material())
    }

    /// 素材 pane を編集している間の入力欄。
    pub(crate) fn arp_materials_input(&self) -> Option<&TextArea<'static>> {
        self.arp_overlay.as_ref()?.materials.as_ref()
    }

    /// 素材リスト・overlay の素材・アルペジエーターの設定が変わって、まだ設定 file へ書いていなければ
    /// true を返し、書いた扱いにする。
    pub fn take_unsaved_settings(&mut self) -> bool {
        std::mem::take(&mut self.settings_unsaved)
    }

    /// 素材を `delta`（+1 次 / -1 前）だけ巡回して選び、鳴らす。末尾の次は先頭。
    /// overlay の素材がリストに無ければ、次は先頭・前は末尾。リストが空なら何もしない。
    pub(super) fn cycle_arp_material(&mut self, delta: isize) -> GuitarArticulationAction {
        let len = self.arp_materials.len();
        if len == 0 {
            return GuitarArticulationAction::Continue;
        }
        let index = match self.arp_material_position() {
            Some(current) => (current as isize + delta).rem_euclid(len as isize) as usize,
            None if delta >= 0 => 0,
            None => len - 1,
        };
        let material = self.arp_materials[index].clone();
        self.select_arp_material(&material)
    }

    /// 素材 pane の編集を始める。カーソルは今の素材の行末、リストに無ければ最後の行末。
    pub(super) fn open_arp_materials(&mut self) {
        let row = self
            .arp_material_position()
            .unwrap_or(self.arp_materials.len().saturating_sub(1));
        let lines = if self.arp_materials.is_empty() {
            vec![String::new()]
        } else {
            self.arp_materials.clone()
        };
        let mut textarea = cmrt_tui_core::text_input::new_multi_line_textarea(lines);
        textarea.move_cursor(CursorMove::Jump(row as u16, 0));
        textarea.move_cursor(CursorMove::End);
        if let Some(overlay) = self.arp_overlay.as_mut() {
            overlay.materials = Some(textarea);
        }
    }

    /// 素材 pane のキー。`Tab` / `Esc` で確定して param pane へ戻り、それ以外は入力欄へ渡す。
    pub(super) fn handle_arp_materials_key(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        let Some(textarea) = self
            .arp_overlay
            .as_mut()
            .and_then(|overlay| overlay.materials.as_mut())
        else {
            return GuitarArticulationAction::Continue;
        };
        if matches!(key.code, KeyCode::Tab | KeyCode::BackTab | KeyCode::Esc) {
            return self.close_arp_materials();
        }
        textarea.input(key);
        GuitarArticulationAction::Continue
    }

    /// 素材 pane を閉じ、空行を除いた行を素材リストにする。カーソル行の素材が overlay の素材と違えば、
    /// それに切り替えて鳴らす。
    fn close_arp_materials(&mut self) -> GuitarArticulationAction {
        let Some(textarea) = self
            .arp_overlay
            .as_mut()
            .and_then(|overlay| overlay.materials.take())
        else {
            return GuitarArticulationAction::Continue;
        };
        let lines: Vec<String> = textarea
            .lines()
            .iter()
            .map(|line| line.trim().to_string())
            .collect();
        let materials: Vec<String> = lines
            .iter()
            .filter(|line| !line.is_empty())
            .cloned()
            .collect();
        if materials != self.arp_materials {
            self.arp_materials = materials;
            self.settings_unsaved = true;
        }
        let cursor_line = &lines[textarea.cursor().0];
        if cursor_line.is_empty() || cursor_line == self.arp_material() {
            return GuitarArticulationAction::Continue;
        }
        let material = cursor_line.clone();
        self.select_arp_material(&material)
    }

    /// `material` を overlay の素材にして鳴らす。解釈できなければ切り替えず、理由を `error` に出す。
    pub(super) fn select_arp_material(&mut self, material: &str) -> GuitarArticulationAction {
        if let Err(reason) = self.rebuild_arp_performance(material) {
            self.error = Some(reason);
            return GuitarArticulationAction::Continue;
        }
        if self.arp_material != material {
            self.arp_material = material.to_string();
            self.settings_unsaved = true;
        }
        self.play(Take::Converted)
    }
}

#[cfg(test)]
mod tests;
