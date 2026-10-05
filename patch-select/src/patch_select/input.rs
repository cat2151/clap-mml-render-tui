//! selector のキー配送。固定モードは候補移動・検索・確定・取消に限定する。

use super::*;
use crossterm::event::KeyEventKind;

impl PatchSelect<'_> {
    pub fn handle_key(&mut self, key: KeyEvent) -> PatchSelectAction {
        if self.drum_kit_only
            && (key.kind == KeyEventKind::Release
                || (key.kind != KeyEventKind::Press
                    && matches!(key.code, KeyCode::Enter | KeyCode::Esc)))
        {
            return PatchSelectAction::Continue;
        }
        if self.filter_editing {
            return self.handle_filter_key(key);
        }
        if self.plugin_menu.is_some() {
            return self.handle_plugin_menu_key(key);
        }
        if let Some(action) = self.handle_auto_reverb_key(key) {
            return action;
        }
        match key.code {
            KeyCode::Esc => return PatchSelectAction::Cancel,
            KeyCode::Enter => {
                return match self.selected() {
                    Some(patch) => PatchSelectAction::Confirm(patch.to_string()),
                    None => PatchSelectAction::Cancel,
                };
            }
            _ => {}
        }
        if is_filter_edit_trigger(key) {
            self.filter_editing = true;
            self.query = text_input::new_single_line_textarea(
                &crate::plugin_menu::condition_to_edit(&self.committed_query),
            );
            return PatchSelectAction::Continue;
        }
        if !self.drum_kit_only && is_plugin_menu_key(key) {
            return self.open_plugin_menu();
        }
        if !self.drum_kit_only && is_preview_key(key) {
            return self.play_selected_line();
        }
        if !self.drum_kit_only && is_add_preset_key(key) {
            return self.add_query_as_preset();
        }
        if !self.drum_kit_only && is_random_jump_key(key) {
            return self.random_jump();
        }
        let navigation_code = if key.modifiers == KeyModifiers::NONE {
            match key.code {
                KeyCode::Char('h') => KeyCode::Left,
                KeyCode::Char('j') => KeyCode::Down,
                KeyCode::Char('k') => KeyCode::Up,
                KeyCode::Char('l') => KeyCode::Right,
                code => code,
            }
        } else {
            key.code
        };
        match navigation_code {
            KeyCode::Left => {
                if self.drum_kit_only {
                    return PatchSelectAction::Continue;
                }
                self.focus = match self.focus {
                    PatchSelectFocus::Groups | PatchSelectFocus::Presets => {
                        PatchSelectFocus::Groups
                    }
                    PatchSelectFocus::Patches => PatchSelectFocus::Presets,
                };
                return PatchSelectAction::Continue;
            }
            KeyCode::Right => {
                if self.drum_kit_only {
                    return PatchSelectAction::Continue;
                }
                self.focus = match self.focus {
                    PatchSelectFocus::Groups => PatchSelectFocus::Presets,
                    PatchSelectFocus::Presets | PatchSelectFocus::Patches => {
                        PatchSelectFocus::Patches
                    }
                };
                return PatchSelectAction::Continue;
            }
            KeyCode::Up => return self.move_focused_cursor(-1),
            KeyCode::Down => return self.move_focused_cursor(1),
            KeyCode::PageUp => return self.move_focused_page(-1),
            KeyCode::PageDown => return self.move_focused_page(1),
            KeyCode::Home => return self.move_focused_to_start(),
            KeyCode::End => return self.move_focused_to_end(),
            _ => {}
        }
        PatchSelectAction::Continue
    }

    /// 絞り込み編集中は selector のキーを動かさず、入力欄だけを操作する。
    /// 候補は入力中にも更新するが、`Esc` なら編集開始時の確定値へ戻す。
    fn handle_filter_key(&mut self, key: KeyEvent) -> PatchSelectAction {
        match key.code {
            KeyCode::Esc => {
                self.filter_editing = false;
                self.query = text_input::new_single_line_textarea(&self.committed_query);
                return self.refilter();
            }
            KeyCode::Enter => {
                self.filter_editing = false;
                self.committed_query = text_input::textarea_value(&self.query);
                return PatchSelectAction::Continue;
            }
            _ => {}
        }
        if !text_input::apply_key_event_to_textarea(&mut self.query, key) {
            return PatchSelectAction::Continue;
        }
        self.refilter()
    }

    /// 選択中の音色で現在行を鳴らす。
    ///
    /// 試聴と同じく音源へこの音色を読み込ませるので、[`Self::previewed`] も進める。
    /// ここを飛ばすと、取り消し時に「読み込ませたのに元へ戻さない」が起きる。
    fn play_selected_line(&mut self) -> PatchSelectAction {
        let Some(patch) = self.selected() else {
            return PatchSelectAction::Continue;
        };
        let patch = patch.to_string();
        self.previewed = Some(patch.clone());
        PatchSelectAction::PlayLine(patch)
    }

    fn random_jump(&mut self) -> PatchSelectAction {
        self.focus = PatchSelectFocus::Patches;
        if self.filtered.len() < 2 {
            return PatchSelectAction::Continue;
        }
        let candidate = cmrt_tui_core::random::random_index(self.filtered.len() - 1)
            .expect("two or more filtered patches have another index");
        self.cursor = if candidate >= self.cursor {
            candidate + 1
        } else {
            candidate
        };
        self.preview_selected()
    }

    fn add_query_as_preset(&mut self) -> PatchSelectAction {
        let value = text_input::textarea_value(&self.query);
        let value = value.trim();
        let destination = self.selected_filter_group().user_destination();
        if value.is_empty()
            || !is_valid_condition(value)
            || patterns_for_role(destination, &self.user_presets)
                .iter()
                .any(|pattern| pattern == value)
        {
            return PatchSelectAction::Continue;
        }
        let previous_len = self.user_presets.len();
        self.user_presets
            .push((destination.key().to_string(), value.to_string()));
        self.user_presets = normalize_user_presets(std::mem::take(&mut self.user_presets));
        if self.user_presets.len() == previous_len {
            return PatchSelectAction::Continue;
        }
        self.role_index = build_role_index(&self.all, &self.user_presets);
        if let Some(panel) = self.auto_reverb.as_mut() {
            panel.set_user_presets(&self.user_presets);
        }
        self.prepared_presets = prepare_presets(
            &self.all,
            &self.user_presets,
            &self.role_index,
            &self.favorites,
            &self.load_measurements,
        );
        let preview = match self.refilter() {
            PatchSelectAction::Preview(patch) => Some(patch),
            _ => None,
        };
        PatchSelectAction::SaveUserPresets {
            presets: self.user_presets.clone(),
            preview,
        }
    }
}
