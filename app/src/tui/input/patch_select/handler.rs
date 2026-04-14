use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::{Mode, PatchSelectPane, TuiApp};

impl<'a> TuiApp<'a> {
    fn add_selected_patch_phrase_favorite(&mut self) {
        let Some(patch_name) = self.patch_select_selected_patch_name() else {
            return;
        };
        let Some(phrase) = self.patch_select_current_phrase() else {
            return;
        };
        self.add_patch_phrase_favorite(patch_name, phrase);
        self.refresh_patch_select_favorites();
        self.sync_patch_select_states();
        self.preview_selected_patch();
    }

    fn handle_patch_select_with_ctrl(&mut self, key_event: KeyEvent) {
        if let KeyCode::Char(c) = key_event.code {
            match c.to_ascii_lowercase() {
                'f' => self.add_selected_patch_phrase_favorite(),
                'j' | 'n' => self.move_patch_select_selection_by(1),
                'k' | 'p' => self.move_patch_select_selection_by(-1),
                's' => self.toggle_patch_select_sort_order(),
                _ => {}
            }
        }
    }

    fn handle_patch_select_filter_input(&mut self, key_event: KeyEvent) {
        crate::text_input::sync_single_line_textarea(
            &mut self.patch_query_textarea,
            &self.patch_query,
        );
        match key_event.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                self.patch_select_filter_active = false;
                self.sync_patch_select_states();
            }
            KeyCode::Backspace if self.patch_query.is_empty() => {
                self.patch_select_filter_active = false;
            }
            KeyCode::Char('?') => self.enter_help(),
            _ => {
                let previous_query = self.patch_query.clone();
                if self.patch_query_textarea.input(key_event) {
                    let next_query = crate::text_input::textarea_value(&self.patch_query_textarea);
                    if next_query == previous_query {
                        return;
                    }
                    self.patch_query = next_query;
                    self.update_patch_filter();
                }
            }
        }
    }

    fn set_patch_select_focus(&mut self, focus: PatchSelectPane) {
        self.patch_select_focus = focus;
        self.sync_patch_select_states();
        self.preview_selected_patch();
    }

    pub(in crate::tui) fn handle_patch_select(&mut self, key_event: KeyEvent) {
        if self.patch_select_filter_active {
            self.handle_patch_select_filter_input(key_event);
            return;
        }

        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
            self.handle_patch_select_with_ctrl(key_event);
            return;
        }

        match key_event.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Char('n') => {
                self.start_notepad_history();
            }
            KeyCode::Char('p') => {
                self.start_patch_phrase_for_patch_name(self.patch_select_selected_patch_name());
            }
            KeyCode::Char('t') => {
                let selected_patch_name = self.patch_select_selected_patch_name();
                self.open_patch_select_overlay(selected_patch_name.as_deref());
            }
            KeyCode::Enter => {
                if let Some(selected) = self.patch_select_selected_patch_name() {
                    self.replace_current_line_patch(&selected);
                    let line = self.lines[self.cursor].clone();
                    self.record_notepad_history(&line);
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.set_patch_select_focus(PatchSelectPane::Patches);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.set_patch_select_focus(PatchSelectPane::Favorites);
            }
            KeyCode::Char('j') | KeyCode::Down => self.move_patch_select_selection_by(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_patch_select_selection_by(-1),
            KeyCode::PageDown => {
                self.move_patch_select_selection_by(self.patch_select_page_size as isize)
            }
            KeyCode::PageUp => {
                self.move_patch_select_selection_by(-(self.patch_select_page_size as isize))
            }
            KeyCode::Char('f') => self.add_selected_patch_phrase_favorite(),
            KeyCode::Char('/') => {
                self.patch_select_focus = PatchSelectPane::Patches;
                self.patch_select_filter_active = true;
                self.patch_query_textarea =
                    crate::text_input::new_single_line_textarea(&self.patch_query);
                self.sync_patch_select_states();
            }
            KeyCode::Char('?') => self.enter_help(),
            _ => {}
        }
    }
}
