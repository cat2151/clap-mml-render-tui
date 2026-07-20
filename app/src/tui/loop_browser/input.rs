use super::*;

impl LoopBrowser {
    pub(in crate::tui) fn handle_key(&mut self, key: KeyCode) -> LoopBrowserAction {
        if self.category_overlay.is_some() {
            return self.handle_category_overlay_key(key);
        }
        match key {
            KeyCode::Esc => LoopBrowserAction::Return,
            KeyCode::Char('q') => LoopBrowserAction::Quit,
            KeyCode::Char('f') => {
                self.toggle_selected_favorite();
                LoopBrowserAction::Continue
            }
            KeyCode::Char('F') => {
                self.toggle_favorites_only();
                LoopBrowserAction::Continue
            }
            KeyCode::Char('c') => {
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
            return LoopBrowserAction::Play(node.path);
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
                expires_at: Instant::now() + FAVORITE_REMOVED_NOTICE_DURATION,
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
}
