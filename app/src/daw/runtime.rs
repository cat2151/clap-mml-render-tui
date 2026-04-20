//! DAW モードのメインループ

use anyhow::Result;
use crossterm::{
    cursor::SetCursorStyle,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::{DawApp, DawExitReason, DawMode, DawNormalAction};

impl DawApp {
    pub(crate) fn uses_textarea_cursor(&self) -> bool {
        match self.mode {
            DawMode::Insert => true,
            DawMode::History => self.history_overlay_filter_active,
            DawMode::PatchSelect => self.patch_select_filter_active,
            DawMode::Normal | DawMode::Help | DawMode::Mixer => false,
        }
    }

    /// TuiApp と同じ terminal を受け取って DAW モードを実行する。
    /// 終了時は `DawExitReason` を返す:
    ///   - `ReturnToTui` : n キーで notepad へ切り替える
    ///   - `QuitApp`     : q キーでアプリを終了する
    pub fn run_with_terminal(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<DawExitReason> {
        struct DeactivateDawHttpServerGuard;

        impl Drop for DeactivateDawHttpServerGuard {
            fn drop(&mut self) {
                super::http_server::deactivate_daw_http_server();
            }
        }

        let _deactivate_daw_http_server_guard = DeactivateDawHttpServerGuard;
        self.kick_all_pending();
        let mut uses_textarea_cursor = self.uses_textarea_cursor();
        execute!(
            std::io::stdout(),
            if uses_textarea_cursor {
                SetCursorStyle::BlinkingBar
            } else {
                SetCursorStyle::DefaultUserShape
            }
        )?;

        loop {
            self.apply_pending_http_commands();
            self.sync_http_status_snapshot();
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

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    use crossterm::event::KeyEventKind;
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        match self.mode {
                            DawMode::Insert => self.handle_insert(key),
                            DawMode::History if self.history_overlay_filter_active => {
                                self.handle_history_overlay_key_event(key)
                            }
                            DawMode::PatchSelect if self.patch_select_filter_active => {
                                self.handle_patch_select_key_event(key)
                            }
                            _ => {}
                        }
                        continue;
                    }

                    match self.mode {
                        DawMode::Normal => match self.handle_normal_key_event(key) {
                            DawNormalAction::ReturnToTui => {
                                self.stop_play();
                                self.save_history_state();
                                return Ok(DawExitReason::ReturnToTui);
                            }
                            DawNormalAction::QuitApp => {
                                self.stop_play();
                                self.save_history_state();
                                return Ok(DawExitReason::QuitApp);
                            }
                            DawNormalAction::Continue => {}
                        },
                        DawMode::Insert => self.handle_insert(key),
                        DawMode::Help => self.handle_help(key.code),
                        DawMode::Mixer => self.handle_mixer(key.code),
                        DawMode::History => self.handle_history_overlay_key_event(key),
                        DawMode::PatchSelect => self.handle_patch_select_key_event(key),
                    }
                }
            }
        }
    }
}
