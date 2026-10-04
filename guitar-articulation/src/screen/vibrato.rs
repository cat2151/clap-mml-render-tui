//! ビブラート設定 overlay（`Shift+V`）。列の ON/OFF と独立に共通設定を変え、値を保持して閉じる。

use crossterm::event::{KeyCode, KeyEvent};

use crate::{PARAM_STEP, VIBRATO_TIME_MAX_MS};

use super::{GuitarArticulationAction, GuitarArticulationScreen, Take};

const ROW_COUNT: usize = 4;
const TIME_STEP_MS: i16 = 50;
const SPEED_CC: u8 = 21;

impl GuitarArticulationScreen {
    /// ビブラート設定で選んでいる行（待機・立ち上がり・最終深さ・速度）。閉じていれば `None`。
    pub fn vibrato_selected(&self) -> Option<usize> {
        self.vibrato_overlay
    }

    pub(super) fn open_vibrato_overlay(&mut self) -> GuitarArticulationAction {
        self.vibrato_overlay = Some(0);
        GuitarArticulationAction::Continue
    }

    /// overlay の間はすべてのキーをここで受け、matrix の列移動やルール切替へ渡さない。
    pub(super) fn handle_vibrato_key(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        let Some(selected) = self.vibrato_overlay else {
            return GuitarArticulationAction::Continue;
        };
        match key.code {
            KeyCode::Esc => self.vibrato_overlay = None,
            KeyCode::Char('j') | KeyCode::Down => {
                self.vibrato_overlay = Some((selected + 1).min(ROW_COUNT - 1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.vibrato_overlay = Some(selected.saturating_sub(1));
            }
            KeyCode::Char('h') | KeyCode::Left => return self.step_vibrato(selected, -1),
            KeyCode::Char('l') | KeyCode::Right => return self.step_vibrato(selected, 1),
            _ => {}
        }
        GuitarArticulationAction::Continue
    }

    fn step_vibrato(&mut self, selected: usize, direction: i16) -> GuitarArticulationAction {
        let changed = if selected == ROW_COUNT - 1 {
            self.rules
                .step_param(SPEED_CC, direction * i16::from(PARAM_STEP))
        } else {
            let mut settings = self.rules.vibrato_settings();
            let step_time = |value| {
                (i32::from(value) + i32::from(direction * TIME_STEP_MS))
                    .clamp(0, i32::from(VIBRATO_TIME_MAX_MS)) as u16
            };
            match selected {
                0 => settings.delay_ms = step_time(settings.delay_ms),
                1 => settings.rise_ms = step_time(settings.rise_ms),
                2 => {
                    settings.depth = (i16::from(settings.depth) + direction * i16::from(PARAM_STEP))
                        .clamp(0, 127) as u8;
                }
                _ => return GuitarArticulationAction::Continue,
            }
            self.rules.set_vibrato_settings(settings)
        };
        if !changed {
            return GuitarArticulationAction::Continue;
        }
        self.rebuild_converted();
        self.record_history();
        self.play(Take::Converted)
    }
}

#[cfg(test)]
mod tests;
