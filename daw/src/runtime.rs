//! DAW モードのメインループ

use anyhow::Result;
use crossterm::{
    cursor::SetCursorStyle,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::{DawApp, DawExitReason, DawMode, DawNormalAction};
use cmrt_mml_overlay::is_mml_overlay_trigger;
use cmrt_tui_core::screen_switch::{
    is_screen_switch_trigger, PrimaryScreen, ScreenSwitchMenuAction,
};

fn target_leaves_workspace(workspace_kind: super::WorkspaceKind, target: PrimaryScreen) -> bool {
    target != workspace_kind.primary_screen()
}

impl DawApp {
    fn leave_for_primary_screen(&mut self, target: PrimaryScreen) -> Option<DawExitReason> {
        if !target_leaves_workspace(self.workspace_kind, target) {
            return None;
        }
        self.stop_play();
        self.save_history_state();
        Some(DawExitReason::SwitchTo {
            target,
            keyboard_patch: (target == PrimaryScreen::Keyboard)
                .then(|| self.current_track_patch_name())
                .flatten(),
        })
    }

    pub(crate) fn uses_textarea_cursor(&self) -> bool {
        match self.mode {
            DawMode::Insert | DawMode::MmlOverlay => true,
            DawMode::History => self.overlays.history.filter_active,
            DawMode::PatchSelect => self.overlays.patch_select.filter_active,
            DawMode::Project => {
                self.overlays.project.action == Some(super::DawProjectFileAction::SaveAs)
                    || self.overlays.project.filter_active
            }
            DawMode::Normal | DawMode::Help | DawMode::Mixer => false,
        }
    }

    /// TuiApp と同じ terminal を受け取って DAW モードを実行する。
    /// 終了時は `DawExitReason` を返す:
    ///   - `ReturnToTui` : n キーで notepad へ切り替える
    ///   - `QuitApp`     : q キーでアプリを終了する
    ///
    /// `autoplay_on_entry` が true の場合、`kick_all_pending()` の直後に
    /// 曲先頭（measure 0）から自動再生を開始する（Shift+P 相当）。
    /// notepad からの `w` 切替や HTTP モード切替では再発火させたくないため、
    /// 真の cold start 呼び出し元のみ true を渡すこと。
    pub fn run_with_terminal(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
        autoplay_on_entry: bool,
    ) -> Result<DawExitReason> {
        struct DeactivateDawHttpServerGuard;

        impl Drop for DeactivateDawHttpServerGuard {
            fn drop(&mut self) {
                super::http_server::deactivate_daw_http_server();
            }
        }

        let _deactivate_daw_http_server_guard = DeactivateDawHttpServerGuard;
        self.kick_all_pending();
        if autoplay_on_entry {
            self.start_play();
        }
        let mut uses_textarea_cursor = self.uses_textarea_cursor();
        execute!(
            std::io::stdout(),
            if uses_textarea_cursor {
                SetCursorStyle::BlinkingBar
            } else {
                SetCursorStyle::DefaultUserShape
            }
        )?;
        // DAW は共有runtimeとは別の描画loopを持つため、entry時に自前で再同期する。
        terminal.clear()?;
        let mut redraw_invalidated = false;

        loop {
            if redraw_invalidated {
                terminal.clear()?;
                redraw_invalidated = false;
            }
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
            self.pump_sound_check_guide();
            self.pump_mml_overlay();
            terminal.draw(|f| self.draw(f))?;

            if event::poll(std::time::Duration::from_millis(50))? {
                let input = event::read()?;
                if matches!(input, Event::Resize(_, _)) {
                    redraw_invalidated = true;
                    continue;
                }
                if let Event::Key(key) = input {
                    use crossterm::event::KeyEventKind;
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if self.overlays.screen_switch.is_open() {
                        if let ScreenSwitchMenuAction::SwitchTo(target) =
                            self.overlays.screen_switch.handle_key(key)
                        {
                            if let Some(exit) = self.leave_for_primary_screen(target) {
                                return Ok(exit);
                            }
                        }
                        continue;
                    }
                    // MML オーバーレイは開いている間キーを総取りする（Ctrl+C も含む）。
                    // 画面切替メニューや Ctrl+C の分岐より先に判定すること。
                    if self.mode == DawMode::MmlOverlay {
                        self.handle_mml_overlay_key_event(key);
                        continue;
                    }
                    if self.mode == DawMode::Normal && is_mml_overlay_trigger(key) {
                        self.try_open_mml_overlay(key);
                        continue;
                    }
                    if self.mode == DawMode::Normal && is_screen_switch_trigger(key) {
                        self.overlays.screen_switch.open();
                        continue;
                    }
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        match self.mode {
                            DawMode::Insert => self.handle_insert(key),
                            DawMode::History if self.overlays.history.filter_active => {
                                self.handle_history_overlay_key_event(key)
                            }
                            DawMode::PatchSelect if self.overlays.patch_select.filter_active => {
                                self.handle_patch_select_key_event(key)
                            }
                            DawMode::Project if self.overlays.project.action.is_some() => {
                                self.handle_project_key_event(key)
                            }
                            _ => {}
                        }
                        continue;
                    }

                    match self.mode {
                        DawMode::Normal => match self.handle_normal_key_event(key) {
                            DawNormalAction::SwitchTo(target) => {
                                if let Some(exit) = self.leave_for_primary_screen(target) {
                                    return Ok(exit);
                                }
                            }
                            DawNormalAction::QuitApp => {
                                self.stop_play();
                                self.save_history_state();
                                return Ok(DawExitReason::QuitApp);
                            }
                            DawNormalAction::EditConfig => {
                                self.stop_play();
                                self.save_history_state();
                                match crate::edit_config_toml(terminal) {
                                    Ok(()) => return Ok(DawExitReason::RestartApp),
                                    Err(error) => self.append_log_line(format!(
                                        "config 編集に失敗しました: {error}"
                                    )),
                                }
                            }
                            DawNormalAction::Continue => {}
                        },
                        DawMode::Insert => self.handle_insert(key),
                        DawMode::Help => self.handle_help(key.code),
                        DawMode::Mixer => self.handle_mixer(key.code),
                        DawMode::History => self.handle_history_overlay_key_event(key),
                        DawMode::PatchSelect => self.handle_patch_select_key_event(key),
                        DawMode::Project => self.handle_project_key_event(key),
                        // 開いている間は上で総取りしているのでここへは来ない。
                        DawMode::MmlOverlay => {}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
