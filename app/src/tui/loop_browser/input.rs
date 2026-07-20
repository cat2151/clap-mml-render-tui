use super::*;

impl LoopBrowser {
    #[cfg(test)]
    pub(in crate::tui) fn handle_key(&mut self, key: KeyCode) -> LoopBrowserAction {
        self.handle_key_event(KeyEvent::new(key, KeyModifiers::NONE))
    }

    pub(in crate::tui) fn handle_key_event(&mut self, key: KeyEvent) -> LoopBrowserAction {
        if self.category_overlay.is_some() {
            return self.handle_category_overlay_key(key.code);
        }
        if key.code == KeyCode::Tab {
            self.focus = match self.focus {
                LoopBrowserPane::Tree => LoopBrowserPane::Tracks,
                LoopBrowserPane::Tracks => LoopBrowserPane::Tree,
            };
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
                    .map(LoopBrowserAction::Trigger)
                    .unwrap_or(LoopBrowserAction::Continue);
            }
        }
        match key.code {
            KeyCode::Esc => LoopBrowserAction::Return,
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
            KeyCode::Char('j') | KeyCode::Down => self.move_cursor(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_cursor(-1),
            KeyCode::PageDown => self.move_cursor(self.page_size as isize),
            KeyCode::PageUp => self.move_cursor(-(self.page_size as isize)),
            KeyCode::Char('l') | KeyCode::Right => self.expand_or_play(),
            KeyCode::Char('h') | KeyCode::Left => {
                self.collapse_or_select_parent();
                LoopBrowserAction::Continue
            }
            KeyCode::Enter | KeyCode::Char(' ') => self.selected_play_action(),
            _ => LoopBrowserAction::Continue,
        }
    }

    fn handle_track_key(&mut self, key: KeyEvent) -> LoopBrowserAction {
        if key.modifiers == KeyModifiers::NONE {
            if let KeyCode::Char(pad @ ('c' | 'd' | 'e' | 'f' | 'g' | 'a' | 'b')) = key.code {
                return self.toggle_current_cell(pad);
            }
        }
        match key.code {
            KeyCode::Esc => LoopBrowserAction::Return,
            KeyCode::Char('q') => LoopBrowserAction::Quit,
            KeyCode::Char('h') | KeyCode::Left => {
                self.measure_cursor = self.measure_cursor.saturating_sub(1);
                LoopBrowserAction::Continue
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.track_cursor = self.track_cursor.saturating_sub(1);
                LoopBrowserAction::Continue
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.move_track_cursor_right();
                LoopBrowserAction::Continue
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_track_cursor_down();
                LoopBrowserAction::Continue
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
                    self.assign_selected_category(category);
                }
            }
            _ => {}
        }
        LoopBrowserAction::Continue
    }

    fn move_cursor(&mut self, delta: isize) -> LoopBrowserAction {
        if self.visible.is_empty() {
            return LoopBrowserAction::Continue;
        }
        let max = self.visible.len().saturating_sub(1) as isize;
        let next = (self.cursor as isize + delta).clamp(0, max) as usize;
        if next == self.cursor {
            return LoopBrowserAction::Continue;
        }
        self.cursor = next;
        self.list_state.select(Some(next));
        self.selected_play_action()
    }

    fn expand_or_play(&mut self) -> LoopBrowserAction {
        let Some(node) = self.visible.get(self.cursor).cloned() else {
            return LoopBrowserAction::Continue;
        };
        if node.is_wav {
            return LoopBrowserAction::Preview(node.path);
        }
        if self.expanded.insert(node.key.clone()) {
            self.rebuild_visible(Some(&node.key));
        }
        LoopBrowserAction::Continue
    }

    fn collapse_or_select_parent(&mut self) {
        let Some(node) = self.visible.get(self.cursor).cloned() else {
            return;
        };
        if !node.is_wav && self.expanded.remove(&node.key) {
            self.rebuild_visible(Some(&node.key));
            return;
        }
        if node.depth == 0 || node.key.components.is_empty() {
            return;
        }
        let mut parent = node.key;
        parent.components.pop();
        self.rebuild_visible(Some(&parent));
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
        if !self.metadata_writable {
            return;
        }
        let selected_path = self.visible.get(self.cursor).map(|node| node.path.clone());
        let previous = self.metadata.clone();
        let added = self.metadata.toggle_favorite(&dir);
        if let Err(error) = self.save_metadata() {
            self.metadata = previous;
            self.metadata_error = Some(format!("お気に入りを保存できません: {error}"));
            return;
        }
        self.metadata_error = None;
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
        if self.category_keys.is_empty() || !self.metadata_writable {
            return;
        }
        self.category_overlay = self.selected_target_dir();
    }

    fn assign_selected_category(&mut self, category: String) {
        let Some(dir) = self.category_overlay.take() else {
            return;
        };
        let selected_path = self.visible.get(self.cursor).map(|node| node.path.clone());
        let previous = self.metadata.clone();
        self.metadata.toggle_category(&dir, &category);
        if let Err(error) = self.save_metadata() {
            self.metadata = previous;
            self.metadata_error = Some(format!("カテゴリを保存できません: {error}"));
            return;
        }
        self.metadata_error = None;
        self.rebuild_visible_for_path(selected_path.as_deref());
    }

    fn save_metadata(&self) -> anyhow::Result<()> {
        match &self.metadata_path {
            Some(path) => self.metadata.save_to(path),
            None => Ok(()),
        }
    }

    fn save_track_grid(&self) -> anyhow::Result<()> {
        match &self.track_grid_path {
            Some(path) => crate::loop_browser_track_grid::save_to(path, &self.track_grid),
            None => Ok(()),
        }
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
        if !self.metadata_writable {
            return LoopBrowserAction::Continue;
        }
        let previous = self.metadata.clone();
        let assigned = self.metadata.toggle_pad(pad, &wav);
        if let Err(error) = self.save_metadata() {
            self.metadata = previous;
            self.metadata_error = Some(format!("WAV padを保存できません: {error}"));
            return LoopBrowserAction::Continue;
        }
        self.metadata_error = None;
        if !assigned {
            self.notice = Some(LoopBrowserNotice {
                text: format!("WAV pad {} を解除しました", pad.to_ascii_uppercase()),
                expires_at: Instant::now() + REMOVED_NOTICE_DURATION,
            });
        }
        LoopBrowserAction::Continue
    }

    fn toggle_current_cell(&mut self, pad: char) -> LoopBrowserAction {
        let Some(wav) = self.metadata.pad(pad).cloned() else {
            return LoopBrowserAction::Continue;
        };
        let audition = wav.path();
        if !self.track_grid_writable {
            return LoopBrowserAction::Trigger(audition);
        }
        let previous = self.track_grid.clone();
        let cell = &mut self.track_grid[self.track_cursor][self.measure_cursor];
        if cell.as_ref().is_some_and(|current| current.matches(&wav)) {
            *cell = None;
        } else {
            *cell = Some(wav);
        }
        if let Err(error) = self.save_track_grid() {
            self.track_grid = previous;
            self.track_grid_error = Some(format!("track listを保存できません: {error}"));
            return LoopBrowserAction::Trigger(audition);
        }
        self.track_grid_error = None;
        LoopBrowserAction::GridChanged {
            audition,
            grid: self.playback_grid(),
        }
    }

    fn move_track_cursor_right(&mut self) {
        let measures = self.track_grid[0].len();
        if self.measure_cursor + 1 < measures {
            self.measure_cursor += 1;
            return;
        }
        if !self.track_grid_writable {
            return;
        }
        let previous = self.track_grid.clone();
        for track in &mut self.track_grid {
            track.push(None);
        }
        if let Err(error) = self.save_track_grid() {
            self.track_grid = previous;
            self.track_grid_error = Some(format!("measure追加を保存できません: {error}"));
            return;
        }
        self.track_grid_error = None;
        self.measure_cursor += 1;
    }

    fn move_track_cursor_down(&mut self) {
        if self.track_cursor + 1 < self.track_grid.len() {
            self.track_cursor += 1;
            return;
        }
        if !self.track_grid_writable {
            return;
        }
        let previous = self.track_grid.clone();
        let measures = self.track_grid[0].len();
        self.track_grid.push(vec![None; measures]);
        if let Err(error) = self.save_track_grid() {
            self.track_grid = previous;
            self.track_grid_error = Some(format!("track追加を保存できません: {error}"));
            return;
        }
        self.track_grid_error = None;
        self.track_cursor += 1;
    }
}
