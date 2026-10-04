//! Keyboardのキー入力を、overlayと通常操作の優先順位に従って処理する。

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{
    plugin_menu, KeyboardAction, KeyboardConnectionPhase, KeyboardContext, KeyboardScreen,
    NumericInputTarget,
};

impl KeyboardScreen<'_> {
    pub fn handle_key(&mut self, key: KeyEvent, ctx: &KeyboardContext<'_>) -> KeyboardAction {
        self.apply_random_chord_progression(Instant::now());
        // 共有の通知は次の Press で閉じる。そのキーは食わずに以下の通常処理へ流す。
        if key.kind == KeyEventKind::Press {
            self.share_notice = None;
        }
        if self.mml_input.is_active() {
            return self.handle_mml_input_key(key);
        }
        if self.patch_filter.is_active() {
            return self.handle_patch_filter_key(key, ctx);
        }
        if self.plugin_menu.is_some() {
            return self.handle_plugin_menu_key(key, ctx);
        }
        if self.effect.is_adding() {
            return self.handle_effect_add_key(key);
        }
        if key.kind == KeyEventKind::Repeat {
            return KeyboardAction::Continue;
        }
        // 数値入力モード中はPressを入力操作として消費する。Releaseだけは通常処理へ
        // 流し、押しっぱなしのノートが鳴りっぱなしになるのを防ぐ。
        if self.state.numeric_input().is_some() && key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char(digit @ '0'..='9') => {
                    self.state.numeric_input_push(digit);
                }
                KeyCode::Backspace => {
                    self.state.numeric_input_backspace();
                }
                KeyCode::Esc => {
                    self.state.cancel_numeric_input();
                }
                KeyCode::Enter => {
                    let message = self.state.confirm_numeric_input();
                    if let Some(message) = message {
                        if self.connection_status().phase.accepts_notes() {
                            if let Some(sender) = &self.midi_sender {
                                sender.send(vec![message], self.state.patch());
                            }
                        }
                    }
                }
                _ => {}
            }
            return KeyboardAction::Continue;
        }
        if let Some(action) = self.handle_help_key(key) {
            return action;
        }
        if let Some(action) = self.handle_effect_pane_key(key) {
            return action;
        }
        if key.kind == KeyEventKind::Press {
            if key.modifiers == KeyModifiers::NONE {
                if let KeyCode::Char(digit @ '0'..='9') = key.code {
                    if self.state.navigation_count.push_digit(digit) {
                        return KeyboardAction::Continue;
                    }
                }
                match key.code {
                    KeyCode::Char('j') => {
                        let delta = self.state.navigation_count.take_delta(1);
                        self.move_focused_cursor(delta, ctx);
                        return KeyboardAction::Continue;
                    }
                    KeyCode::Char('k') => {
                        let delta = self.state.navigation_count.take_delta(-1);
                        self.move_focused_cursor(delta, ctx);
                        return KeyboardAction::Continue;
                    }
                    KeyCode::Char('l') => {
                        let delta = self.state.navigation_count.take_delta(1);
                        self.move_focus(delta, ctx);
                        return KeyboardAction::Continue;
                    }
                    KeyCode::Char('h') => {
                        let delta = self.state.navigation_count.take_delta(-1);
                        self.move_focus(delta, ctx);
                        return KeyboardAction::Continue;
                    }
                    _ => {}
                }
            } else if key.modifiers == KeyModifiers::CONTROL {
                match key.code {
                    KeyCode::Char('d') => {
                        let delta = self.state.navigation_count.take_delta(10);
                        self.move_focused_cursor(delta, ctx);
                        return KeyboardAction::Continue;
                    }
                    KeyCode::Char('u') => {
                        let delta = self.state.navigation_count.take_delta(-10);
                        self.move_focused_cursor(delta, ctx);
                        return KeyboardAction::Continue;
                    }
                    _ => {}
                }
            }
            self.state.navigation_count.clear();
        }
        if key.kind == KeyEventKind::Press
            && matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT)
            && key.code == KeyCode::Char('/')
        {
            self.open_patch_filter(ctx);
            return KeyboardAction::Continue;
        }
        if plugin_menu::is_plugin_menu_key(key) {
            self.open_plugin_menu(ctx);
            return KeyboardAction::Continue;
        }
        if key.kind == KeyEventKind::Press
            && key.modifiers == KeyModifiers::SHIFT
            && matches!(key.code, KeyCode::Char('i' | 'I'))
        {
            self.toggle_random_chord_mode();
            return KeyboardAction::Continue;
        }
        if key.kind == KeyEventKind::Press
            && key.modifiers == KeyModifiers::SHIFT
            && matches!(key.code, KeyCode::Char('h' | 'H'))
        {
            let multiplier = self.state.cycle_buffer_multiplier();
            if let Some(sender) = &self.midi_sender {
                sender.set_buffer_multiplier(multiplier);
            }
            return KeyboardAction::Continue;
        }
        if key.kind == KeyEventKind::Press
            && key.modifiers == KeyModifiers::SHIFT
            && matches!(key.code, KeyCode::Char('z' | 'Z'))
        {
            if self.connection_status().phase.accepts_notes() {
                let message = self.state.toggle_cc_periodic(Instant::now());
                if let Some(sender) = &self.midi_sender {
                    sender.send(vec![message], self.state.patch());
                }
            }
            return KeyboardAction::Continue;
        }
        if key.kind == KeyEventKind::Press && key.modifiers == KeyModifiers::NONE {
            match key.code {
                KeyCode::Down => {
                    self.move_focused_cursor(1, ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::Up => {
                    self.move_focused_cursor(-1, ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::Right => {
                    self.move_focus(1, ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::Left => {
                    self.move_focus(-1, ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::PageDown => {
                    self.move_focused_cursor(10, ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::PageUp => {
                    self.move_focused_cursor(-10, ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::End => {
                    self.move_focused_to_end(ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::Home => {
                    self.move_focused_to_start(ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('v') => {
                    self.state.cycle_velocity(Instant::now());
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('m') => {
                    if self.connection_status().phase.accepts_notes() {
                        let message = self.state.cycle_modulation(Instant::now());
                        if let Some(sender) = &self.midi_sender {
                            sender.send(vec![message], self.state.patch());
                        }
                    }
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('p') => {
                    if self.connection_status().phase.accepts_notes() {
                        let message = self.state.cycle_pitch_bend(Instant::now());
                        if let Some(sender) = &self.midi_sender {
                            sender.send(vec![message], self.state.patch());
                        }
                    }
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('t') => {
                    if self.connection_status().phase.accepts_notes() {
                        let messages = self.cycle_keyboard_note_playback(Instant::now());
                        self.send_after_cancel(messages);
                    }
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('y') => {
                    self.copy_share_command();
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('i') => {
                    self.mml_input.open();
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('x') => {
                    self.state.begin_numeric_input(NumericInputTarget::CcNumber);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('z') => {
                    self.state.begin_numeric_input(NumericInputTarget::CcValue);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('r')
                    if matches!(
                        self.connection_status().phase,
                        KeyboardConnectionPhase::Error(_)
                    ) =>
                {
                    self.discard_random_chord_progression();
                    self.state.take_reset_messages();
                    self.prepare_connection(ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('r') => {
                    self.select_random_patch(ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('n') => {
                    self.finish();
                    return KeyboardAction::ReturnToNotepad;
                }
                KeyCode::Char('w') => {
                    self.finish();
                    return KeyboardAction::LaunchDaw;
                }
                KeyCode::Char('q') => {
                    self.finish();
                    return KeyboardAction::Quit;
                }
                _ => {}
            }
        }
        self.handle_note_key(key)
    }

    fn handle_mml_input_key(&mut self, key: KeyEvent) -> KeyboardAction {
        if key.kind == KeyEventKind::Release {
            self.release_note_while_typing(key);
            return KeyboardAction::Continue;
        }

        match key.code {
            KeyCode::Esc => self.mml_input.cancel(),
            KeyCode::Enter => {
                if let Some(progression) = self.mml_input.confirm() {
                    self.random_chord.enabled = false;
                    self.discard_random_chord_progression();
                    let ready = self.connection_status().phase.accepts_notes();
                    let messages =
                        self.state
                            .replace_repeat_chords(progression, Instant::now(), ready);
                    if ready {
                        self.send_after_cancel(messages);
                    }
                }
            }
            _ => self.mml_input.input(key),
        }
        KeyboardAction::Continue
    }
}
