//! 入力欄・各 overlay を優先し、matrix のキー操作へ振り分ける。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::ui::{PARAM_LIST_KEY, ROW_RULE_ROWS, RULE_LIST_KEY, RULE_ROWS};

use super::{history, GuitarArticulationAction, GuitarArticulationScreen, Take};

impl GuitarArticulationScreen {
    pub fn handle_key_event(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        // ヘルプを開いている間は閉じるキーだけを見る。
        if self.help_open {
            if is_help_key(key) || key.code == KeyCode::Esc {
                self.help_open = false;
            }
            return GuitarArticulationAction::Continue;
        }
        // overlay の絞り込みは正規表現で `?` を打てるので、`?` の扱いも overlay に任せる。
        if self.effect_overlay.is_some() {
            return self.handle_effect_key(key);
        }
        if self.history_overlay.is_some() {
            return self.handle_history_key(key);
        }
        if self.sample_midi_list.is_some() {
            return self.handle_sample_midi_list_key(key);
        }
        if self.rule_list.is_some() {
            return self.handle_rule_list_key(key);
        }
        if self.param_list.is_some() {
            return self.handle_param_list_key(key);
        }
        if self.vibrato_overlay.is_some() {
            return self.handle_vibrato_key(key);
        }
        if self.arp_overlay.is_some() {
            return self.handle_arp_overlay_key(key);
        }
        if self.smf.input_open() {
            return self.handle_smf_input_key(key);
        }
        // MML は `?` を使わないので、入力欄を開いていても `?` はヘルプへ回す。
        if is_help_key(key) {
            self.help_open = true;
            return GuitarArticulationAction::Continue;
        }
        if self.input.is_some() {
            return self.handle_input_key(key);
        }
        self.error = None;
        if self.sample_midi.is_some() {
            return self.handle_sample_midi_key(key);
        }
        if let Some(action) = self.handle_smf_key(key) {
            return action;
        }
        if history::is_history_key(key) {
            return self.open_history_overlay();
        }
        if is_shift_key(key, 'V') {
            return self.open_vibrato_overlay();
        }
        if is_shift_key(key, 'R') {
            return self.toggle_repeat();
        }
        if is_shift_key(key, 'A') {
            return self.cycle_accent_pattern();
        }
        match key.code {
            KeyCode::Char('i') => {
                self.open_input();
                GuitarArticulationAction::Continue
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                self.play_cursor_note()
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.cursor + 1 < self.column_count() {
                    self.cursor += 1;
                }
                self.play_cursor_note()
            }
            KeyCode::Char('o') => GuitarArticulationAction::OpenSampleMidiList,
            KeyCode::Char('x') => self.open_effect_overlay(),
            KeyCode::Char('w') => self.toggle_effect_dry(),
            KeyCode::Char('n') => {
                self.note_preview = !self.note_preview;
                GuitarArticulationAction::Continue
            }
            KeyCode::Char('f') => {
                self.startup_instrument = self.startup_instrument.toggled();
                GuitarArticulationAction::SaveSettings
            }
            KeyCode::Char('b') => self.play(Take::Plain),
            KeyCode::Char(' ') => self.play(Take::Converted),
            KeyCode::Char('q') => GuitarArticulationAction::Quit,
            KeyCode::Char(RULE_LIST_KEY) => self.open_rule_list(),
            KeyCode::Char(PARAM_LIST_KEY) => self.open_param_list(),
            KeyCode::Char('z') => self.open_arp_overlay(),
            KeyCode::Char(ch) => {
                if let Some(rule_row) = RULE_ROWS.iter().find(|rule_row| rule_row.key == ch) {
                    self.toggle_rule(rule_row.rule)
                } else if let Some((rule, _, _)) =
                    ROW_RULE_ROWS.iter().find(|(_, key, _)| *key == ch)
                {
                    self.toggle_row_rule(*rule)
                } else {
                    GuitarArticulationAction::Continue
                }
            }
            _ => GuitarArticulationAction::Continue,
        }
    }
}

pub(super) fn is_shift_key(key: KeyEvent, upper: char) -> bool {
    key.code == KeyCode::Char(upper)
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

fn is_help_key(key: KeyEvent) -> bool {
    key.code == KeyCode::Char('?')
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}
