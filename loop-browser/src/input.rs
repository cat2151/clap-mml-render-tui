use super::*;

impl LoopBrowser {
    #[cfg(any(test, feature = "test-support"))]
    pub fn handle_key(&mut self, key: KeyCode) -> LoopBrowserAction {
        self.handle_key_event(KeyEvent::new(key, KeyModifiers::NONE))
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> LoopBrowserAction {
        // 絞り込み入力中はすべてのキーが入力欄へ入る（`?` も数字も Tab も文字扱い）。
        // navigation_count と focus の処理より前に返すこと。
        if self.filter_input.is_some() {
            return self.handle_filter_input_key(key);
        }
        if self.bpm_input.is_some() {
            return self.handle_bpm_input_key(key);
        }
        if self.help_overlay.is_some() {
            self.navigation_count.clear();
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')
            ) {
                self.help_overlay = None;
            }
            return LoopBrowserAction::Continue;
        }
        if self.mixer_overlay_open {
            self.navigation_count.clear();
            return self.handle_mixer_overlay_key(key.code);
        }
        if self.category_overlay.is_some() {
            self.navigation_count.clear();
            return self.handle_category_overlay_key(key.code);
        }
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('b') {
            self.navigation_count.clear();
            self.bpm_input = Some(cmrt_tui_core::bpm::BpmInput::default());
            return LoopBrowserAction::Continue;
        }
        if key.code == KeyCode::Char('?') {
            self.navigation_count.clear();
            self.help_overlay = Some(self.focus);
            cmrt_tui_core::memory::request_refresh();
            return LoopBrowserAction::Continue;
        }
        if key.modifiers == KeyModifiers::NONE && key.code == KeyCode::Char('p') {
            self.navigation_count.clear();
            self.playback_paused = !self.playback_paused;
            return LoopBrowserAction::SetPlaybackPaused {
                paused: self.playback_paused,
                start_measure: self.measure_cursor,
            };
        }
        if key.modifiers == KeyModifiers::SHIFT && key.code == KeyCode::Char('O') {
            self.navigation_count.clear();
            self.toggle_auto_random();
            return LoopBrowserAction::Continue;
        }
        if key.modifiers == KeyModifiers::NONE {
            if let KeyCode::Char(digit @ '0'..='9') = key.code {
                if self.navigation_count.push_digit(digit) {
                    return LoopBrowserAction::Continue;
                }
            }
        }
        if key.modifiers != KeyModifiers::NONE
            || !matches!(key.code, KeyCode::Char('h' | 'j' | 'k' | 'l'))
        {
            self.navigation_count.clear();
        }
        if key.code == KeyCode::Tab {
            self.focus = match self.focus {
                LoopBrowserPane::Tree => LoopBrowserPane::Tracks,
                LoopBrowserPane::Tracks => LoopBrowserPane::Tree,
            };
            if self.focus == LoopBrowserPane::Tracks {
                self.sync_tree_to_current_cell();
            }
            return LoopBrowserAction::Continue;
        }
        match self.focus {
            LoopBrowserPane::Tree => self.handle_tree_key(key),
            LoopBrowserPane::Tracks => self.handle_track_key(key),
        }
    }

    fn handle_tree_key(&mut self, key: KeyEvent) -> LoopBrowserAction {
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            if let KeyCode::Char(character) = key.code {
                let pad = character.to_ascii_lowercase();
                if PAD_KEYS.contains(&pad) {
                    return self.toggle_selected_pad(pad);
                }
            }
        }
        if key.modifiers == KeyModifiers::NONE {
            if let KeyCode::Char(pad @ ('c' | 'd' | 'e' | 'f' | 'g' | 'a' | 'b')) = key.code {
                return self
                    .pad_path(pad)
                    .map(|path| LoopBrowserAction::Trigger { pad, path })
                    .unwrap_or(LoopBrowserAction::Continue);
            }
        }
        match key.code {
            KeyCode::Esc => LoopBrowserAction::Continue,
            KeyCode::Char('q') => LoopBrowserAction::Quit,
            KeyCode::Char('v') => {
                self.toggle_selected_favorite();
                LoopBrowserAction::Continue
            }
            KeyCode::Char('V') => {
                self.toggle_favorites_only();
                LoopBrowserAction::Continue
            }
            KeyCode::Char('t') => {
                self.open_category_overlay();
                LoopBrowserAction::Continue
            }
            KeyCode::Char('/')
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                self.open_filter_input();
                LoopBrowserAction::Continue
            }
            KeyCode::Char('r') if key.modifiers == KeyModifiers::NONE => self.select_random_wav(),
            KeyCode::Char('j') => {
                let delta = self.navigation_count.take_delta(1);
                self.move_cursor(delta)
            }
            KeyCode::Char('k') => {
                let delta = self.navigation_count.take_delta(-1);
                self.move_cursor(delta)
            }
            KeyCode::Down => self.move_cursor(1),
            KeyCode::Up => self.move_cursor(-1),
            KeyCode::PageDown => self.move_cursor(10),
            KeyCode::PageUp => self.move_cursor(-10),
            KeyCode::Char('l') => {
                self.navigation_count.take_delta(1);
                self.expand_or_play()
            }
            KeyCode::Right => self.expand_or_play(),
            KeyCode::Char('h') => {
                let count = self.navigation_count.take_delta(1) as usize;
                for _ in 0..count {
                    if !self.collapse_or_select_parent() {
                        break;
                    }
                }
                LoopBrowserAction::Continue
            }
            KeyCode::Left => {
                self.collapse_or_select_parent();
                LoopBrowserAction::Continue
            }
            KeyCode::Enter | KeyCode::Char(' ') => self.selected_play_action(),
            _ => LoopBrowserAction::Continue,
        }
    }

    fn handle_track_key(&mut self, key: KeyEvent) -> LoopBrowserAction {
        if key.modifiers == KeyModifiers::ALT {
            return match key.code {
                KeyCode::Up => self.move_current_track(-1),
                KeyCode::Down => self.move_current_track(1),
                _ => LoopBrowserAction::Continue,
            };
        }
        if key.modifiers == KeyModifiers::SHIFT {
            if let KeyCode::Char(character) = key.code {
                return match character.to_ascii_lowercase() {
                    'r' => self.randomize_current_measure(),
                    'm' => self.randomize_track_solos(),
                    _ => LoopBrowserAction::Continue,
                };
            }
        }
        if key.modifiers == KeyModifiers::NONE {
            if let KeyCode::Char(pad @ ('c' | 'd' | 'e' | 'f' | 'g' | 'a' | 'b')) = key.code {
                return self.toggle_current_cell(pad);
            }
        }
        match key.code {
            KeyCode::Esc => LoopBrowserAction::Continue,
            KeyCode::Char('q') => LoopBrowserAction::Quit,
            KeyCode::Char('m') if key.modifiers == KeyModifiers::NONE => {
                self.mixer_cursor_track = self
                    .track_cursor
                    .min(self.track_grid.len().saturating_sub(1));
                self.mixer_overlay_open = true;
                LoopBrowserAction::Continue
            }
            KeyCode::Char('s') if key.modifiers == KeyModifiers::NONE => {
                self.toggle_current_track_solo()
            }
            KeyCode::Char('r') if key.modifiers == KeyModifiers::NONE => self.select_random_wav(),
            KeyCode::Char('h') => {
                let count = self.navigation_count.take_delta(1) as usize;
                self.measure_cursor = self.measure_cursor.saturating_sub(count);
                self.sync_tree_to_current_cell();
                LoopBrowserAction::Continue
            }
            KeyCode::Left => {
                self.measure_cursor = self.measure_cursor.saturating_sub(1);
                self.sync_tree_to_current_cell();
                LoopBrowserAction::Continue
            }
            KeyCode::Char('k') => {
                let count = self.navigation_count.take_delta(1) as usize;
                self.track_cursor = self.track_cursor.saturating_sub(count);
                self.sync_tree_to_current_cell();
                LoopBrowserAction::Continue
            }
            KeyCode::Up => {
                self.track_cursor = self.track_cursor.saturating_sub(1);
                self.sync_tree_to_current_cell();
                LoopBrowserAction::Continue
            }
            KeyCode::Char('l') => {
                let count = self.navigation_count.take_delta(1) as usize;
                self.move_track_cursor_right(count);
                self.sync_tree_to_current_cell();
                LoopBrowserAction::Continue
            }
            KeyCode::Right => {
                self.move_track_cursor_right(1);
                self.sync_tree_to_current_cell();
                LoopBrowserAction::Continue
            }
            KeyCode::Char('j') => {
                let count = self.navigation_count.take_delta(1) as usize;
                self.move_track_cursor_down(count);
                self.sync_tree_to_current_cell();
                LoopBrowserAction::Continue
            }
            KeyCode::Down => {
                self.move_track_cursor_down(1);
                self.sync_tree_to_current_cell();
                LoopBrowserAction::Continue
            }
            _ => LoopBrowserAction::Continue,
        }
    }
}
