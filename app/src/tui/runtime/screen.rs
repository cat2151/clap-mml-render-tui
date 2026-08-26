use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{backend::Backend, backend::CrosstermBackend, Terminal};

use super::super::{Mode, PlayState, TuiApp};
use crate::screen_switch::{is_screen_switch_trigger, PrimaryScreen, ScreenSwitchMenuAction};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) enum DawEntryRoute {
    Restored(PrimaryScreen),
    ScreenSwitch(PrimaryScreen),
    Keyboard,
    Notepad,
    Http,
}

impl DawEntryRoute {
    pub(in crate::tui) const fn screen(self) -> Option<PrimaryScreen> {
        match self {
            Self::Restored(screen) | Self::ScreenSwitch(screen) if screen.is_daw() => Some(screen),
            Self::Keyboard | Self::Notepad | Self::Http => Some(PrimaryScreen::Daw),
            Self::Restored(_) | Self::ScreenSwitch(_) => None,
        }
    }
}

pub(in crate::tui) const fn daw_workspace_for_screen(
    screen: PrimaryScreen,
) -> Option<crate::daw::WorkspaceKind> {
    match screen {
        PrimaryScreen::DailyDaw => Some(crate::daw::WorkspaceKind::Daily),
        PrimaryScreen::Daw => Some(crate::daw::WorkspaceKind::Persistent),
        _ => None,
    }
}

pub(in crate::tui) fn clear_terminal_for_new_screen<B: Backend>(
    terminal: &mut Terminal<B>,
    rendered_screen: &mut Option<PrimaryScreen>,
    active_screen: PrimaryScreen,
) -> std::result::Result<(), B::Error> {
    if *rendered_screen == Some(active_screen) {
        return Ok(());
    }
    terminal.clear()?;
    *rendered_screen = Some(active_screen);
    Ok(())
}

pub(super) enum DawRunOutcome {
    Continue,
    Quit,
    Restart,
}

impl<'a> TuiApp<'a> {
    pub(in crate::tui) fn can_open_screen_switch_menu(&self) -> bool {
        match self.active_screen {
            PrimaryScreen::Notepad => self.notepad.mode == Mode::Normal,
            PrimaryScreen::DailyDaw | PrimaryScreen::Daw => false,
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

    /// MML オーバーレイへ音源インスタンスを明け渡すため、いまの画面の演奏を止める。
    ///
    /// 画面はそのまま残るので、開いている入力欄やヘルプは閉じない
    /// （画面を離れる [`Self::leave_active_screen`] との違いはそこ）。
    pub(in crate::tui) fn stop_active_screen_playback(&mut self) {
        match self.active_screen {
            PrimaryScreen::Notepad => self.stop_notepad_playback(),
            PrimaryScreen::Keyboard => self.finish_keyboard(),
            PrimaryScreen::LoopBrowser => self.stop_loop_browser(),
            PrimaryScreen::GridSequencer => self.stop_grid_sequencer_playback(),
            PrimaryScreen::DailyDaw | PrimaryScreen::Daw => {}
        }
    }

    /// [`Self::stop_active_screen_playback`] で止めた演奏を再開する。
    ///
    /// MML オーバーレイを閉じたときに使う。画面は離れていないので、いた場所から
    /// 演奏だけが戻る（grid sequencer なら同じ grid のまま鳴り直す）。
    pub(in crate::tui) fn resume_active_screen_playback(&mut self) {
        match self.active_screen {
            PrimaryScreen::Keyboard => self.resume_keyboard(),
            PrimaryScreen::LoopBrowser => self.begin_loop_browser_startup(),
            PrimaryScreen::GridSequencer => self.enter_grid_sequencer(),
            // notepad と DAW は明示的に再生する画面なので、勝手に鳴らし始めない。
            PrimaryScreen::Notepad | PrimaryScreen::DailyDaw | PrimaryScreen::Daw => {}
        }
    }

    fn leave_active_screen(&mut self) {
        match self.active_screen {
            PrimaryScreen::Notepad => self.stop_notepad_playback(),
            PrimaryScreen::Keyboard => self.finish_keyboard(),
            PrimaryScreen::LoopBrowser => self.stop_loop_browser(),
            PrimaryScreen::GridSequencer => self.finish_grid_sequencer(),
            PrimaryScreen::DailyDaw | PrimaryScreen::Daw => {}
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
            PrimaryScreen::DailyDaw | PrimaryScreen::Daw => {
                self.notepad.mode = Mode::Normal;
                self.active_screen = target;
            }
            PrimaryScreen::Keyboard => {
                if source == PrimaryScreen::Notepad {
                    self.start_keyboard_from_notepad();
                } else if source.is_daw() {
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
        entry_route: DawEntryRoute,
        autoplay_on_entry: bool,
    ) -> Result<DawRunOutcome> {
        let mut requested_screen = entry_route
            .screen()
            .expect("run_daw_screen requires a DAW entry route");
        let mut autoplay = autoplay_on_entry;
        loop {
            let workspace_kind = daw_workspace_for_screen(requested_screen)
                .expect("run_daw_screen requires a DAW primary screen");
            self.switch_to_primary_screen(requested_screen, None);
            self.save_notepad_and_session_state();

            let mut daw = crate::daw::DawApp::new_for_workspace(
                std::sync::Arc::clone(&self.cfg),
                self.plugin_entries.clone(),
                workspace_kind,
            );
            let outcome = daw.run_with_terminal(terminal, autoplay);
            drop(daw);

            match outcome? {
                crate::daw::DawExitReason::SwitchTo {
                    target,
                    keyboard_patch: _,
                } if target.is_daw() => {
                    requested_screen = target;
                    autoplay = false;
                }
                crate::daw::DawExitReason::SwitchTo {
                    target,
                    keyboard_patch,
                } => {
                    self.switch_to_primary_screen(target, keyboard_patch);
                    return Ok(DawRunOutcome::Continue);
                }
                crate::daw::DawExitReason::QuitApp => return Ok(DawRunOutcome::Quit),
                crate::daw::DawExitReason::RestartApp => return Ok(DawRunOutcome::Restart),
            }
        }
    }
}

#[cfg(test)]
mod tests;
