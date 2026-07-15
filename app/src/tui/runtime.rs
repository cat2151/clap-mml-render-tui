use anyhow::Result;
use crossterm::{
    cursor::SetCursorStyle,
    event::{
        self, Event, KeyCode, KeyModifiers, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use std::sync::Arc;

use super::keyboard::KeyboardAction;
use super::{Mode, NormalAction, PlayState, TuiApp, TuiExitReason};

struct TerminalCleanup {
    raw_mode_enabled: bool,
    alternate_screen_enabled: bool,
    keyboard_enhancement_enabled: bool,
}

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        let _ = execute!(std::io::stdout(), SetCursorStyle::DefaultUserShape);
        if self.keyboard_enhancement_enabled {
            let _ = execute!(std::io::stdout(), PopKeyboardEnhancementFlags);
        }
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
            Mode::Normal | Mode::NotepadHistoryGuide | Mode::Help | Mode::Keyboard => false,
        }
    }

    pub fn run(&mut self) -> Result<TuiExitReason> {
        crate::daw::ensure_http_server_for_mode_switch();
        enable_raw_mode()?;
        let mut cleanup = TerminalCleanup {
            raw_mode_enabled: true,
            alternate_screen_enabled: false,
            keyboard_enhancement_enabled: false,
        };
        let mut stdout = std::io::stdout();
        if matches!(
            crossterm::terminal::supports_keyboard_enhancement(),
            Ok(true)
        ) {
            execute!(
                stdout,
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
            )?;
            cleanup.keyboard_enhancement_enabled = true;
        }
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

        // 真の cold start（プロセス起動直後）かどうか。DAW⇔notepad のモード切替では
        // 自動再生を再発火させたくないため、この判定は一度だけ行う。
        let started_in_notepad_mode = !self.is_daw_mode && self.mode == Mode::Normal;

        // 前回 DAW モードで終了していた場合は直接 DAW モードで起動する
        let mut quit_from_startup_daw = false;
        let mut restart_from_startup_daw = false;
        if self.is_daw_mode {
            let mut daw = crate::daw::DawApp::new(Arc::clone(&self.cfg), self.entry_ptr);
            match daw.run_with_terminal(&mut terminal, self.cfg.autoplay_on_startup)? {
                crate::daw::DawExitReason::ReturnToTui => {
                    self.is_daw_mode = false;
                }
                crate::daw::DawExitReason::LaunchKeyboard { patch } => self.start_keyboard(patch),
                crate::daw::DawExitReason::QuitApp => {
                    quit_from_startup_daw = true;
                }
                crate::daw::DawExitReason::RestartApp => {
                    restart_from_startup_daw = true;
                }
            }
        }
        self.prepare_restored_keyboard_connection();

        loop {
            if quit_from_startup_daw {
                break;
            }
            if restart_from_startup_daw {
                self.flush_patch_phrase_store_if_dirty();
                self.save_history_state();
                self.flush_notepad_disk_cache();
                return Ok(TuiExitReason::RestartApp);
            }
            if crate::daw::take_http_mode_switch_request() {
                if self.mode == Mode::Keyboard {
                    self.finish_keyboard();
                }
                self.flush_patch_phrase_store_if_dirty();
                self.save_history_state();
                self.flush_notepad_disk_cache();
                let mut daw = crate::daw::DawApp::new(Arc::clone(&self.cfg), self.entry_ptr);
                match daw.run_with_terminal(&mut terminal, false)? {
                    crate::daw::DawExitReason::ReturnToTui => {
                        self.is_daw_mode = false;
                    }
                    crate::daw::DawExitReason::LaunchKeyboard { patch } => {
                        self.start_keyboard(patch)
                    }
                    crate::daw::DawExitReason::QuitApp => {
                        self.is_daw_mode = true;
                        break;
                    }
                    crate::daw::DawExitReason::RestartApp => {
                        self.is_daw_mode = true;
                        self.flush_patch_phrase_store_if_dirty();
                        self.save_history_state();
                        self.flush_notepad_disk_cache();
                        return Ok(TuiExitReason::RestartApp);
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
                *self.known_disk_cache_hashes.lock().unwrap() =
                    crate::tui::disk_cache::scan_valid_cache_hashes(self.cfg.sample_rate as u32);
                self.hydrate_all_lines_from_disk_cache_at_startup();
                self.prime_normal_mode_startup_cache();
                if started_in_notepad_mode && self.cfg.autoplay_on_startup {
                    if let Some(mml) = self
                        .lines
                        .get(self.cursor)
                        .map(|line| line.trim().to_string())
                        .filter(|mml| !mml.is_empty())
                    {
                        self.kick_play(mml);
                    }
                }
                self.startup_normal_cache_primed = true;
            }

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    use crossterm::event::KeyEventKind;
                    if self.mode == Mode::Keyboard {
                        match self.handle_keyboard_key_event(key) {
                            KeyboardAction::Continue | KeyboardAction::ReturnToNotepad => {}
                            KeyboardAction::Quit => break,
                            KeyboardAction::LaunchDaw => {
                                self.flush_patch_phrase_store_if_dirty();
                                self.save_history_state();
                                self.flush_notepad_disk_cache();
                                let mut daw =
                                    crate::daw::DawApp::new(Arc::clone(&self.cfg), self.entry_ptr);
                                match daw.run_with_terminal(&mut terminal, false)? {
                                    crate::daw::DawExitReason::ReturnToTui => {
                                        self.mode = Mode::Normal;
                                        self.is_daw_mode = false;
                                    }
                                    crate::daw::DawExitReason::LaunchKeyboard { patch } => {
                                        self.start_keyboard(patch);
                                    }
                                    crate::daw::DawExitReason::QuitApp => {
                                        self.is_daw_mode = true;
                                        break;
                                    }
                                    crate::daw::DawExitReason::RestartApp => {
                                        self.is_daw_mode = true;
                                        self.flush_patch_phrase_store_if_dirty();
                                        self.save_history_state();
                                        self.flush_notepad_disk_cache();
                                        return Ok(TuiExitReason::RestartApp);
                                    }
                                }
                            }
                        }
                        continue;
                    }
                    // keyboard以外はPressのみ処理。Release/Repeatは無視（二重発火防止）。
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
                                self.flush_notepad_disk_cache();
                                let mut daw =
                                    crate::daw::DawApp::new(Arc::clone(&self.cfg), self.entry_ptr);
                                match daw.run_with_terminal(&mut terminal, false)? {
                                    crate::daw::DawExitReason::ReturnToTui => {
                                        self.mode = Mode::Normal;
                                        self.is_daw_mode = false;
                                    }
                                    crate::daw::DawExitReason::LaunchKeyboard { patch } => {
                                        self.start_keyboard(patch);
                                    }
                                    crate::daw::DawExitReason::QuitApp => {
                                        self.is_daw_mode = true;
                                        break;
                                    }
                                    crate::daw::DawExitReason::RestartApp => {
                                        self.is_daw_mode = true;
                                        self.flush_patch_phrase_store_if_dirty();
                                        self.save_history_state();
                                        self.flush_notepad_disk_cache();
                                        return Ok(TuiExitReason::RestartApp);
                                    }
                                }
                            }
                            NormalAction::LaunchKeyboard => self.start_keyboard_from_notepad(),
                            NormalAction::EditConfig => {
                                let session = self.begin_playback_session();
                                self.set_play_state_if_current(session, PlayState::Idle);
                                match crate::config_editor::edit_config_toml(&mut terminal) {
                                    Ok(()) => {
                                        self.flush_patch_phrase_store_if_dirty();
                                        self.save_history_state();
                                        self.flush_notepad_disk_cache();
                                        return Ok(TuiExitReason::RestartApp);
                                    }
                                    Err(error) => {
                                        *self.play_state.lock().unwrap() = PlayState::Err(format!(
                                            "config 編集に失敗しました: {error}"
                                        ));
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
                        Mode::Keyboard => unreachable!("keyboard input is handled above"),
                    }
                }
            }
        }

        // 終了前にセッション状態を保存する（端末クリーンアップの成否に関わらず実行）。
        // 保存失敗はベストエフォートとして無視する（終了処理のため通知手段がない）。
        self.flush_patch_phrase_store_if_dirty();
        self.save_history_state();
        self.flush_notepad_disk_cache();
        Ok(TuiExitReason::Quit)
    }
}
