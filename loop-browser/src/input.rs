use super::*;

impl LoopBrowser {
    #[cfg(any(test, feature = "test-support"))]
    pub fn handle_key(&mut self, key: KeyCode) -> LoopBrowserAction {
        self.handle_key_event(KeyEvent::new(key, KeyModifiers::NONE))
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> LoopBrowserAction {
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
        if key.code == KeyCode::Char('?') {
            self.navigation_count.clear();
            self.help_overlay = Some(self.focus);
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

    fn handle_mixer_overlay_key(&mut self, key: KeyCode) -> LoopBrowserAction {
        match key {
            KeyCode::Esc => {
                self.mixer_overlay_open = false;
                LoopBrowserAction::Continue
            }
            KeyCode::Char('h') | KeyCode::Left if self.mixer_cursor_track > 0 => {
                self.mixer_cursor_track -= 1;
                LoopBrowserAction::Continue
            }
            KeyCode::Char('l') | KeyCode::Right
                if self.mixer_cursor_track + 1 < self.track_grid.len() =>
            {
                self.mixer_cursor_track += 1;
                LoopBrowserAction::Continue
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.adjust_mixer_volume(-cmrt_tui_core::mixer::MIXER_STEP_DB)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.adjust_mixer_volume(cmrt_tui_core::mixer::MIXER_STEP_DB)
            }
            _ => LoopBrowserAction::Continue,
        }
    }

    fn handle_category_overlay_key(&mut self, key: KeyCode) -> LoopBrowserAction {
        match key {
            KeyCode::Esc => self.category_overlay = None,
            KeyCode::Char(key) => {
                let key = key.to_ascii_lowercase();
                if let Some(category) = self
                    .category_keys
                    .iter()
                    .find(|(candidate, _)| *candidate == key)
                    .map(|(_, category)| category.clone())
                {
                    if self.assign_selected_category(category) {
                        return LoopBrowserAction::GridRefresh {
                            grid: self.playback_grid(),
                            reason: LoopGridChange::Category,
                        };
                    }
                }
            }
            _ => {}
        }
        LoopBrowserAction::Continue
    }

    fn move_cursor(&mut self, delta: isize) -> LoopBrowserAction {
        let started_at = Instant::now();
        if self.visible.is_empty() {
            return LoopBrowserAction::Continue;
        }
        let previous = self.cursor;
        let max = self.visible.len().saturating_sub(1) as isize;
        let next = (self.cursor as isize).saturating_add(delta).clamp(0, max) as usize;
        if next == self.cursor {
            return LoopBrowserAction::Continue;
        }
        self.cursor = next;
        let trace_id = super::performance::next_trace_id();
        self.pending_render_trace = Some(trace_id);
        let selected_is_wav = self.visible[next].is_wav;
        let action = self.selected_play_action_with_trace(trace_id);
        super::performance::log_cursor_move(
            trace_id,
            started_at.elapsed(),
            previous,
            next,
            self.visible.len(),
            if selected_is_wav { "wav" } else { "directory" },
            selected_is_wav,
        );
        action
    }

    fn expand_or_play(&mut self) -> LoopBrowserAction {
        let Some(node) = self.visible.get(self.cursor).cloned() else {
            return LoopBrowserAction::Continue;
        };
        if node.is_wav {
            let trace_id = super::performance::next_trace_id();
            self.pending_preview_trace = Some(trace_id);
            return LoopBrowserAction::Preview(node.path);
        }
        if self.expanded.insert(node.key.clone()) {
            self.rebuild_visible(Some(&node.key));
        }
        LoopBrowserAction::Continue
    }

    fn collapse_or_select_parent(&mut self) -> bool {
        let Some(node) = self.visible.get(self.cursor).cloned() else {
            return false;
        };
        if !node.is_wav && self.expanded.remove(&node.key) {
            self.rebuild_visible(Some(&node.key));
            return true;
        }
        if node.depth == 0 || node.key.components.is_empty() {
            return false;
        }
        let mut parent = node.key;
        parent.components.pop();
        self.rebuild_visible(Some(&parent));
        true
    }

    fn selected_target_dir(&self) -> Option<LoopDirId> {
        let node = self.visible.get(self.cursor)?;
        let root_path = &self.roots.get(node.key.root)?.0;
        let mut components = node.key.components.clone();
        if node.is_wav {
            components.pop();
        }
        let relative = components.iter().collect::<PathBuf>();
        Some(LoopDirId::new(root_path, &relative))
    }

    fn toggle_selected_favorite(&mut self) {
        let Some(dir) = self.selected_target_dir() else {
            return;
        };
        if !self.metadata.writable {
            return;
        }
        let selected_path = self.visible.get(self.cursor).map(|node| node.path.clone());
        let Some(added) = self.metadata.try_mutate(
            |metadata| metadata.toggle_favorite(&dir),
            |path, metadata| metadata.save_to(path),
            "お気に入りを保存できません",
        ) else {
            return;
        };
        self.rebuild_favorite_wav_keys();
        if !added {
            self.notice = Some(LoopBrowserNotice {
                text: "お気に入りdirを解除しました".to_string(),
                expires_at: Instant::now() + REMOVED_NOTICE_DURATION,
            });
        }
        self.rebuild_visible_for_path(selected_path.as_deref());
    }

    fn toggle_favorites_only(&mut self) {
        let selected_path = self.visible.get(self.cursor).map(|node| node.path.clone());
        self.favorites_only = !self.favorites_only;
        self.category_overlay = None;
        self.rebuild_visible_for_path(selected_path.as_deref());
    }

    fn open_category_overlay(&mut self) {
        if self.category_keys.is_empty() || !self.metadata.writable {
            return;
        }
        self.category_overlay = self.selected_target_dir();
    }

    fn assign_selected_category(&mut self, category: String) -> bool {
        let Some(dir) = self.category_overlay.take() else {
            return false;
        };
        let selected_path = self.visible.get(self.cursor).map(|node| node.path.clone());
        if self
            .metadata
            .try_mutate(
                |metadata| metadata.toggle_category(&dir, &category),
                |path, metadata| metadata.save_to(path),
                "カテゴリを保存できません",
            )
            .is_none()
        {
            return false;
        }
        self.rebuild_wav_categories();
        self.rebuild_visible_for_path(selected_path.as_deref());
        true
    }

    fn selected_wav_id(&self) -> Option<LoopWavId> {
        let node = self.visible.get(self.cursor)?;
        if !node.is_wav {
            return None;
        }
        let root = &self.roots.get(node.key.root)?.0;
        let relative = node.key.components.iter().collect::<PathBuf>();
        Some(LoopWavId::new(root, &relative))
    }

    fn toggle_selected_pad(&mut self, pad: char) -> LoopBrowserAction {
        let Some(wav) = self.selected_wav_id() else {
            return LoopBrowserAction::Continue;
        };
        if !self.metadata.writable {
            return LoopBrowserAction::Continue;
        }
        let Some(assigned) = self.metadata.try_mutate(
            |metadata| metadata.toggle_pad(pad, &wav),
            |path, metadata| metadata.save_to(path),
            "WAV padを保存できません",
        ) else {
            return LoopBrowserAction::Continue;
        };
        if !assigned {
            self.notice = Some(LoopBrowserNotice {
                text: format!("WAV pad {} を解除しました", pad.to_ascii_uppercase()),
                expires_at: Instant::now() + REMOVED_NOTICE_DURATION,
            });
        } else {
            self.notice = None;
        }
        LoopBrowserAction::Continue
    }
}
