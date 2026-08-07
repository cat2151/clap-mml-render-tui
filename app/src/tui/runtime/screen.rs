use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{backend::CrosstermBackend, Terminal};

use super::super::{Mode, PlayState, TuiApp};
use crate::screen_switch::{is_screen_switch_trigger, PrimaryScreen, ScreenSwitchMenuAction};

pub(super) enum DawRunOutcome {
    Continue,
    Quit,
    Restart,
}

impl<'a> TuiApp<'a> {
    pub(in crate::tui) fn can_open_screen_switch_menu(&self) -> bool {
        match self.active_screen {
            PrimaryScreen::Notepad => self.notepad.mode == Mode::Normal,
            PrimaryScreen::Daw => false,
            PrimaryScreen::Keyboard => {
                !self.keyboard.mml_input.is_active()
                    && self.keyboard.state.numeric_input().is_none()
            }
            PrimaryScreen::LoopBrowser => {
                self.loop_browser.state.help_overlay.is_none()
                    && !self.loop_browser.state.mixer_overlay_open
                    && self.loop_browser.state.category_overlay.is_none()
            }
            PrimaryScreen::GridSequencer => !self.grid_sequencer.help_open,
        }
    }

    pub(in crate::tui) fn try_open_screen_switch_menu(&mut self, key: KeyEvent) -> bool {
        if self.can_open_screen_switch_menu() && is_screen_switch_trigger(key) {
            if self.active_screen == PrimaryScreen::GridSequencer {
                self.grid_sequencer.cancel_mouse_gesture();
            }
            self.screen_switch_menu.open();
            true
        } else {
            false
        }
    }

    pub(in crate::tui) fn handle_screen_switch_menu_key(
        &mut self,
        key: KeyEvent,
    ) -> Option<PrimaryScreen> {
        match self.screen_switch_menu.handle_key(key) {
            ScreenSwitchMenuAction::SwitchTo(target) if target != self.active_screen => {
                Some(target)
            }
            ScreenSwitchMenuAction::Continue
            | ScreenSwitchMenuAction::Closed
            | ScreenSwitchMenuAction::SwitchTo(_) => None,
        }
    }

    fn stop_notepad_playback(&self) {
        let session = self.playback_session.begin();
        self.playback_session
            .set_play_state_if_current(session, PlayState::Idle);
    }

    fn leave_active_screen(&mut self) {
        match self.active_screen {
            PrimaryScreen::Notepad => self.stop_notepad_playback(),
            PrimaryScreen::Keyboard => self.finish_keyboard(),
            PrimaryScreen::LoopBrowser => self.stop_loop_browser(),
            PrimaryScreen::GridSequencer => self.finish_grid_sequencer(),
            PrimaryScreen::Daw => {}
        }
    }

    pub(in crate::tui) fn switch_to_primary_screen(
        &mut self,
        target: PrimaryScreen,
        keyboard_patch: Option<String>,
    ) {
        // HTTP要求などmenu外からの遷移でも、復帰先へoverlay状態を持ち越さない。
        self.screen_switch_menu.close();
        if target == self.active_screen {
            return;
        }
        let source = self.active_screen;
        self.leave_active_screen();
        match target {
            PrimaryScreen::Notepad => {
                self.notepad.mode = Mode::Normal;
                self.active_screen = PrimaryScreen::Notepad;
                self.notepad.reset_sound_check_guide();
            }
            PrimaryScreen::Daw => {
                self.notepad.mode = Mode::Normal;
                self.active_screen = PrimaryScreen::Daw;
            }
            PrimaryScreen::Keyboard => {
                if source == PrimaryScreen::Notepad {
                    self.start_keyboard_from_notepad();
                } else if source == PrimaryScreen::Daw {
                    self.start_keyboard(keyboard_patch);
                } else {
                    self.resume_keyboard();
                }
            }
            PrimaryScreen::LoopBrowser => self.begin_loop_browser_startup(),
            PrimaryScreen::GridSequencer => self.enter_grid_sequencer(),
        }
    }

    pub(super) fn run_daw_screen(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
        autoplay_on_entry: bool,
    ) -> Result<DawRunOutcome> {
        self.switch_to_primary_screen(PrimaryScreen::Daw, None);
        self.save_notepad_and_session_state();

        let mut daw = crate::daw::DawApp::new(std::sync::Arc::clone(&self.cfg), self.entry_ptr);
        match daw.run_with_terminal(terminal, autoplay_on_entry)? {
            crate::daw::DawExitReason::SwitchTo {
                target,
                keyboard_patch,
            } => {
                self.switch_to_primary_screen(target, keyboard_patch);
                Ok(DawRunOutcome::Continue)
            }
            crate::daw::DawExitReason::QuitApp => Ok(DawRunOutcome::Quit),
            crate::daw::DawExitReason::RestartApp => Ok(DawRunOutcome::Restart),
        }
    }
}
