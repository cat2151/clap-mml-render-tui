//! 画面の「MML と設定のセット」の履歴。状態が変わる操作の直後に今の状態を先頭へ積み、
//! 積んだ 1 件を画面の状態へ戻す。演奏と試聴では積まない。
//!
//! history overlay（`Shift+H`）は選んだ 1 件を画面の状態に当てたまま試聴させ、
//! `Enter` でそのまま確定、`Esc` / `Shift+H` で開く直前の状態へ戻す。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::history::{GuitarArticulationHistory, GuitarArticulationHistoryEntry};
use crate::{notes_from_events, ColumnRuleAnchor};

use super::{GuitarArticulationAction, GuitarArticulationScreen, Take};

/// 開いている history overlay。`before` は開く直前の状態で、取り消しで戻す先。
pub(super) struct HistoryOverlay {
    selected: usize,
    before: GuitarArticulationHistoryEntry,
}

/// history overlay を開くキー、兼、取り消して閉じるキー。
pub(super) fn is_history_key(key: KeyEvent) -> bool {
    key.code == KeyCode::Char('H') && key.modifiers.contains(KeyModifiers::SHIFT)
}

impl GuitarArticulationScreen {
    /// 保存済みの履歴を持たせ、先頭の 1 件（前回終了時の状態）を画面の状態にする。
    pub fn with_history(mut self, history: GuitarArticulationHistory) -> Self {
        if let Some(latest) = history.entries.first() {
            self.apply_history_entry(latest);
        }
        self.history = history;
        self
    }

    /// 新しい順の履歴。
    pub fn history(&self) -> &GuitarArticulationHistory {
        &self.history
    }

    /// 今の状態（MML・ルール・確定済みの chain・列ルールの付け替え元）を履歴 1 件にする。
    pub fn history_entry(&self) -> GuitarArticulationHistoryEntry {
        GuitarArticulationHistoryEntry {
            mml: self.mml.clone(),
            rules: self.rules.clone(),
            effect_chain: self.effect_chain.clone(),
            anchor: self.anchor.clone(),
        }
    }

    /// 履歴 1 件を画面の状態にする。列ルールは同じ MML の列番号なので消さずに当てる。
    /// 付け替え元の無い entry は、entry の MML と列ルールを付け替え元にする（列ルールが無ければ持たない）。
    /// MML を解釈できなければ状態を変えず、理由を `error` に出す。
    pub fn apply_history_entry(&mut self, entry: &GuitarArticulationHistoryEntry) {
        let plain = if entry.mml.is_empty() {
            Vec::new()
        } else {
            match cmrt_chord::timed_performance(&entry.mml) {
                Ok(performance) => performance.events,
                Err(reason) => {
                    self.error = Some(reason);
                    return;
                }
            }
        };
        self.mml = entry.mml.clone();
        self.notes = notes_from_events(&plain);
        self.plain = plain;
        self.rules = entry.rules.clone();
        self.effect_chain = entry.effect_chain.clone();
        self.anchor = entry.anchor.clone().or_else(|| {
            (!entry.rules.is_empty()).then(|| ColumnRuleAnchor::new(&entry.mml, &entry.rules))
        });
        self.cursor = self.cursor.min(self.column_count().saturating_sub(1));
        self.error = None;
        self.rebuild_converted();
    }

    /// history overlay で選んでいる履歴の位置。閉じていれば `None`。
    pub fn history_overlay_selected(&self) -> Option<usize> {
        self.history_overlay
            .as_ref()
            .map(|overlay| overlay.selected)
    }

    /// 今の状態を先頭へ積んで overlay を開き、履歴の保存を求める。
    pub(super) fn open_history_overlay(&mut self) -> GuitarArticulationAction {
        self.record_history();
        self.history_overlay = Some(HistoryOverlay {
            selected: 0,
            before: self.history_entry(),
        });
        GuitarArticulationAction::SaveHistory
    }

    /// overlay を開いている間のキー。ここで全部受け、画面のキーへは渡さない。
    pub(super) fn handle_history_key(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        let Some(mut overlay) = self.history_overlay.take() else {
            return GuitarArticulationAction::Continue;
        };
        if is_history_key(key) || key.code == KeyCode::Esc {
            self.apply_history_entry(&overlay.before);
            return GuitarArticulationAction::Continue;
        }
        if key.code == KeyCode::Enter {
            self.record_history();
            return GuitarArticulationAction::SaveHistory;
        }
        let selected = match key.code {
            KeyCode::Char('j') | KeyCode::Down => overlay.selected + 1,
            KeyCode::Char('k') | KeyCode::Up => overlay.selected.wrapping_sub(1),
            _ => overlay.selected,
        };
        let action = match self.history.entries.get(selected).cloned() {
            Some(entry) if selected != overlay.selected => {
                overlay.selected = selected;
                self.apply_history_entry(&entry);
                if self.error.is_none() && !self.plain.is_empty() {
                    GuitarArticulationAction::Play(Take::Converted)
                } else {
                    GuitarArticulationAction::Continue
                }
            }
            _ => GuitarArticulationAction::Continue,
        };
        self.history_overlay = Some(overlay);
        action
    }

    /// 今の状態を履歴の先頭へ積む。
    pub(super) fn record_history(&mut self) {
        let entry = self.history_entry();
        self.history.push_front(entry);
    }
}

#[cfg(test)]
mod tests;
