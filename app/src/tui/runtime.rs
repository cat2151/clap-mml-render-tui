use anyhow::Result;
use crossterm::{
    cursor::SetCursorStyle,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use std::sync::Arc;

use super::{Mode, NormalAction, TuiApp};

struct TerminalCleanup {
    raw_mode_enabled: bool,
    alternate_screen_enabled: bool,
}

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        let _ = execute!(std::io::stdout(), SetCursorStyle::DefaultUserShape);
        if self.alternate_screen_enabled {
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        }
        if self.raw_mode_enabled {
            let _ = disable_raw_mode();
        }
    }
}

impl<'a> TuiApp<'a> {
    pub(crate) fn uses_textarea_cursor(&self) -> bool {
        match self.mode {
            Mode::Insert => true,
            Mode::PatchSelect => self.patch_select_filter_active,
            Mode::NotepadHistory => self.notepad_filter_active,
            Mode::PatchPhrase => self.patch_phrase_filter_active,
            Mode::Normal | Mode::NotepadHistoryGuide | Mode::Help => false,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        crate::daw::ensure_http_server_for_mode_switch();
        enable_raw_mode()?;
        let mut cleanup = TerminalCleanup {
            raw_mode_enabled: true,
            alternate_screen_enabled: false,
        };
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        cleanup.alternate_screen_enabled = true;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        let mut uses_textarea_cursor = self.uses_textarea_cursor();
        execute!(
            std::io::stdout(),
            if uses_textarea_cursor {
                SetCursorStyle::BlinkingBar
            } else {
                SetCursorStyle::DefaultUserShape
            }
        )?;

        // 前回 DAW モードで終了していた場合は直接 DAW モードで起動する
        let mut quit_from_startup_daw = false;
        if self.is_daw_mode {
            let mut daw = crate::daw::DawApp::new(Arc::clone(&self.cfg), self.entry_ptr);
            match daw.run_with_terminal(&mut terminal)? {
                crate::daw::DawExitReason::ReturnToTui => {
                    self.is_daw_mode = false;
                }
                crate::daw::DawExitReason::QuitApp => {
                    quit_from_startup_daw = true;
                }
            }
        }

        loop {
            if quit_from_startup_daw {
                break;
            }
            if crate::daw::take_http_mode_switch_request() {
                self.flush_patch_phrase_store_if_dirty();
                self.save_history_state();
                let mut daw = crate::daw::DawApp::new(Arc::clone(&self.cfg), self.entry_ptr);
                match daw.run_with_terminal(&mut terminal)? {
                    crate::daw::DawExitReason::ReturnToTui => {
                        self.is_daw_mode = false;
                    }
                    crate::daw::DawExitReason::QuitApp => {
                        self.is_daw_mode = true;
                        break;
                    }
                }
                continue;
            }
            let next_uses_textarea_cursor = self.uses_textarea_cursor();
            if next_uses_textarea_cursor != uses_textarea_cursor {
                execute!(
                    std::io::stdout(),
                    if next_uses_textarea_cursor {
                        SetCursorStyle::BlinkingBar
                    } else {
                        SetCursorStyle::DefaultUserShape
                    }
                )?;
                uses_textarea_cursor = next_uses_textarea_cursor;
            }
            terminal.draw(|f| self.draw(f))?;
            if !self.startup_normal_cache_primed && self.mode == Mode::Normal {
                self.prime_normal_mode_startup_cache();
                self.startup_normal_cache_primed = true;
            }

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    // Press のみ処理。Release/Repeat は無視（二重発火防止）
                    use crossterm::event::KeyEventKind;
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        match self.mode {
                            Mode::Insert => self.handle_insert(key),
                            Mode::PatchSelect if self.patch_select_filter_active => {
                                self.handle_patch_select(key)
                            }
                            Mode::NotepadHistory if self.notepad_filter_active => {
                                self.handle_notepad_history_key_event(key)
                            }
                            Mode::PatchPhrase if self.patch_phrase_filter_active => {
                                self.handle_patch_phrase_key_event(key)
                            }
                            _ => {}
                        }
                        continue;
                    }
                    match self.mode {
                        Mode::Normal => match self.handle_normal_key_event(key) {
                            NormalAction::Quit => break,
                            NormalAction::LaunchDaw => {
                                self.flush_patch_phrase_store_if_dirty();
                                self.save_history_state();
                                let mut daw =
                                    crate::daw::DawApp::new(Arc::clone(&self.cfg), self.entry_ptr);
                                match daw.run_with_terminal(&mut terminal)? {
                                    crate::daw::DawExitReason::ReturnToTui => {
                                        self.is_daw_mode = false;
                                    }
                                    crate::daw::DawExitReason::QuitApp => {
                                        self.is_daw_mode = true;
                                        break;
                                    }
                                }
                            }
                            NormalAction::Continue => {}
                        },
                        Mode::Insert => self.handle_insert(key),
                        Mode::PatchSelect => self.handle_patch_select(key),
                        Mode::NotepadHistory => self.handle_notepad_history_key_event(key),
                        Mode::PatchPhrase => self.handle_patch_phrase_key_event(key),
                        Mode::NotepadHistoryGuide => self.handle_notepad_history_guide(key.code),
                        Mode::Help => self.handle_help(key.code),
                    }
                }
            }
        }

        // 終了前にセッション状態を保存する（端末クリーンアップの成否に関わらず実行）。
        // 保存失敗はベストエフォートとして無視する（終了処理のため通知手段がない）。
        self.flush_patch_phrase_store_if_dirty();
        self.save_history_state();
        Ok(())
    }
}
