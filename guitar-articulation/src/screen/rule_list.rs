//! 奏法リスト overlay（`t`）。カーソル列の列ルールを全部並べ、j/k で選び Enter で ON/OFF する。
//! 切り替えても overlay は開いたままで、Esc で閉じる。

use crossterm::event::{KeyCode, KeyEvent};

use crate::ui::RULE_ROWS;

use super::{GuitarArticulationAction, GuitarArticulationScreen};

impl GuitarArticulationScreen {
    /// 奏法リストで選んでいる行（[`RULE_ROWS`] の位置）。閉じていれば `None`。
    pub fn rule_list_selected(&self) -> Option<usize> {
        self.rule_list
    }

    pub(super) fn open_rule_list(&mut self) -> GuitarArticulationAction {
        if self.column_count() == 0 {
            self.error = Some("i で MML を入力してください".to_string());
        } else {
            self.rule_list = Some(0);
        }
        GuitarArticulationAction::Continue
    }

    pub(super) fn handle_rule_list_key(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        let Some(selected) = self.rule_list else {
            return GuitarArticulationAction::Continue;
        };
        match key.code {
            KeyCode::Esc => self.rule_list = None,
            KeyCode::Char('j') | KeyCode::Down => {
                self.rule_list = Some((selected + 1).min(RULE_ROWS.len() - 1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.rule_list = Some(selected.saturating_sub(1));
            }
            KeyCode::Enter => return self.toggle_rule(RULE_ROWS[selected].0),
            _ => {}
        }
        GuitarArticulationAction::Continue
    }
}
