use cmrt_patch_select::{PatchSelectAction, PAGE_STEP};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::Mode;
use crate::NotepadScreen;

/// 試聴後の先読みで優先する向き。音色一覧を上下に動かすキーだけが向きを持つ。
fn navigation_delta(key_event: KeyEvent) -> Option<isize> {
    match key_event.code {
        KeyCode::Down | KeyCode::Char('j') => Some(1),
        KeyCode::Up | KeyCode::Char('k') => Some(-1),
        KeyCode::PageDown => Some(PAGE_STEP),
        KeyCode::PageUp => Some(-PAGE_STEP),
        _ => None,
    }
}

impl<'a> NotepadScreen<'a> {
    pub(crate) fn handle_patch_select(&mut self, key_event: KeyEvent) {
        let Some(select) = self.patch_select.as_ref() else {
            self.mode = Mode::Normal;
            return;
        };
        if select.captures_all_keys() {
            self.forward_key_to_patch_select(key_event);
            return;
        }

        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char(c) = key_event.code {
                let code = match c.to_ascii_lowercase() {
                    'f' => return self.add_selected_patch_phrase_favorite(),
                    'j' | 'n' => KeyCode::Down,
                    'k' | 'p' => KeyCode::Up,
                    _ => return,
                };
                self.forward_key_to_patch_select(KeyEvent::new(code, KeyModifiers::NONE));
            }
            return;
        }

        match key_event.code {
            KeyCode::Char('n') => {
                self.patch_select = None;
                self.start_notepad_history();
            }
            KeyCode::Char('p') => {
                let selected_patch_name = self.patch_select_selected_patch_name();
                self.patch_select = None;
                self.start_patch_phrase_for_patch_name(selected_patch_name);
            }
            KeyCode::Char('t') => {
                let selected_patch_name = self.patch_select_selected_patch_name();
                self.open_patch_select_overlay(selected_patch_name.as_deref());
            }
            KeyCode::Char('f') => self.add_selected_patch_phrase_favorite(),
            KeyCode::Char('?') => self.enter_help(),
            _ => self.forward_key_to_patch_select(key_event),
        }
    }

    fn forward_key_to_patch_select(&mut self, key_event: KeyEvent) {
        let Some(select) = self.patch_select.as_mut() else {
            return;
        };
        let action = select.handle_key(key_event);
        self.apply_patch_select_action(action, navigation_delta(key_event));
    }

    fn apply_patch_select_action(&mut self, action: PatchSelectAction, delta: Option<isize>) {
        match action {
            PatchSelectAction::Continue => {}
            PatchSelectAction::Preview(_) | PatchSelectAction::PlayLine(_) => {
                self.preview_selected_patch_with_navigation_hint(delta);
            }
            PatchSelectAction::Confirm(patch_name) => {
                let auto_reverb = self
                    .patch_select
                    .as_ref()
                    .and_then(|select| select.auto_reverb_stage(&patch_name));
                self.replace_current_line_patch(&patch_name, auto_reverb);
                let line = self.editor.lines[self.editor.cursor].clone();
                self.record_notepad_history(&line);
                self.close_patch_select();
                self.prime_normal_mode_startup_cache();
            }
            // notepad は selector に音色を読み込ませていないので、戻す音色は無い。
            PatchSelectAction::Cancel => self.close_patch_select(),
            PatchSelectAction::SaveAutoReverb { rules, preview } => {
                let settings = cmrt_history::AutoReverbSettings {
                    enabled: rules.enabled(),
                    rules: rules.to_saved(),
                };
                if let Err(error) = cmrt_history::save_auto_reverb_settings(&settings) {
                    *self.playback.session.play_state().lock().unwrap() =
                        crate::PlayState::Err(format!("auto reverb の保存に失敗: {error}"));
                }
                // 同じ音色でも、掛ける reverb が変わったので鳴らし直す。
                if preview.is_some() {
                    self.preview_selected_patch();
                }
            }
            PatchSelectAction::SaveUserPresets { presets, preview } => {
                if let Err(error) = cmrt_history::save_mml_patch_filter_presets(&presets) {
                    *self.playback.session.play_state().lock().unwrap() =
                        crate::PlayState::Err(format!("preset の保存に失敗: {error}"));
                }
                if preview.is_some() {
                    self.preview_selected_patch();
                }
            }
        }
    }

    fn close_patch_select(&mut self) {
        self.patch_select = None;
        self.mode = Mode::Normal;
    }
}
