//! パラメータ overlay（`u`）。行全体のパラメータ（[`PARAMS`]）を並べ、j/k で選び h/l で
//! [`PARAM_STEP`] ずつ変える。値が変わるたびに Articulated を演奏する。Esc で閉じる。

use crossterm::event::{KeyCode, KeyEvent};

use crate::{PARAMS, PARAM_STEP};

use super::{GuitarArticulationAction, GuitarArticulationScreen, Take};

impl GuitarArticulationScreen {
    /// パラメータ overlay で選んでいる行（[`PARAMS`] の位置）。閉じていれば `None`。
    pub fn param_list_selected(&self) -> Option<usize> {
        self.param_list
    }

    pub(super) fn open_param_list(&mut self) -> GuitarArticulationAction {
        self.param_list = Some(0);
        GuitarArticulationAction::Continue
    }

    pub(super) fn handle_param_list_key(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        let Some(selected) = self.param_list else {
            return GuitarArticulationAction::Continue;
        };
        let step = i16::from(PARAM_STEP);
        match key.code {
            KeyCode::Esc => self.param_list = None,
            KeyCode::Char('j') | KeyCode::Down => {
                self.param_list = Some((selected + 1).min(PARAMS.len() - 1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.param_list = Some(selected.saturating_sub(1));
            }
            KeyCode::Char('h') | KeyCode::Left => return self.step_param(selected, -step),
            KeyCode::Char('l') | KeyCode::Right => return self.step_param(selected, step),
            _ => {}
        }
        GuitarArticulationAction::Continue
    }

    /// 選んでいるパラメータを `delta` だけ変え、Articulated を作り直して鳴らす。端で変わらなければ何もしない。
    fn step_param(&mut self, selected: usize, delta: i16) -> GuitarArticulationAction {
        if !self.rules.step_param(PARAMS[selected].cc, delta) {
            return GuitarArticulationAction::Continue;
        }
        self.rebuild_converted();
        self.record_history();
        self.play(Take::Converted)
    }
}
