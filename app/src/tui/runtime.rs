use anyhow::Result;
use crossterm::{
    cursor::SetCursorStyle,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::Backend, backend::CrosstermBackend, layout::Rect, Terminal};

use super::grid_sequencer::GridSequencerAction;
use super::keyboard::KeyboardAction;
use super::loop_browser::LoopBrowserAction;
use super::{NormalAction, PlayState, PrimaryScreen, TuiApp, TuiExitReason};

mod screen;

#[cfg(test)]
mod tests;

use screen::DawRunOutcome;

struct TerminalCleanup {
    raw_mode_enabled: bool,
    alternate_screen_enabled: bool,
    keyboard_enhancement_enabled: bool,
    mouse_capture_enabled: bool,
}

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        if self.mouse_capture_enabled {
            let _ = execute!(std::io::stdout(), DisableMouseCapture);
        }
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

fn sync_mouse_capture(enabled: &mut bool, requested: bool) -> Result<()> {
    if *enabled == requested {
        return Ok(());
    }
    if requested {
        execute!(std::io::stdout(), EnableMouseCapture)?;
    } else {
        execute!(std::io::stdout(), DisableMouseCapture)?;
    }
    *enabled = requested;
    Ok(())
}

fn clear_terminal_for_new_screen<B: Backend>(
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

impl<'a> TuiApp<'a> {
    pub(crate) fn uses_mouse_capture(&self) -> bool {
        self.active_screen == PrimaryScreen::GridSequencer
    }

    pub(crate) fn uses_textarea_cursor(&self) -> bool {
        match self.active_screen {
            PrimaryScreen::Keyboard => self.keyboard.mml_input.is_active(),
            PrimaryScreen::LoopBrowser | PrimaryScreen::GridSequencer => false,
            PrimaryScreen::Notepad | PrimaryScreen::Daw => self.notepad.uses_textarea_cursor(),
        }
    }

    pub fn run(&mut self) -> Result<TuiExitReason> {
        crate::daw::ensure_http_server_for_mode_switch();
        enable_raw_mode()?;
        let mut cleanup = TerminalCleanup {
            raw_mode_enabled: true,
            alternate_screen_enabled: false,
            keyboard_enhancement_enabled: false,
            mouse_capture_enabled: false,
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
        // Terminal の初期bufferは空画面を前提にするが、実端末のalternate screenには
        // 前回内容が残る実装もある。初回と画面切替時だけ物理画面ごと消去する。
        let mut rendered_screen = None;
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
        let started_in_notepad_mode =
            self.active_screen == crate::screen_switch::PrimaryScreen::Notepad;

        // 前回 DAW モードで終了していた場合は直接 DAW モードで起動する
        let mut quit_from_startup_daw = false;
        let mut restart_from_startup_daw = false;
        if self.active_screen == crate::screen_switch::PrimaryScreen::Daw {
            match self.run_daw_screen(&mut terminal, self.cfg.autoplay_on_startup)? {
                DawRunOutcome::Continue => {}
                DawRunOutcome::Quit => quit_from_startup_daw = true,
                DawRunOutcome::Restart => restart_from_startup_daw = true,
            }
        }
        self.prepare_restored_keyboard_connection();
        self.enter_restored_grid_sequencer();
        sync_mouse_capture(
            &mut cleanup.mouse_capture_enabled,
            self.uses_mouse_capture(),
        )?;

        loop {
            if quit_from_startup_daw {
                break;
            }
            if restart_from_startup_daw {
                self.save_notepad_and_session_state();
                return Ok(TuiExitReason::RestartApp);
            }
            if crate::daw::take_http_mode_switch_request() {
                sync_mouse_capture(&mut cleanup.mouse_capture_enabled, false)?;
                rendered_screen = None;
                match self.run_daw_screen(&mut terminal, false)? {
                    DawRunOutcome::Continue => {}
                    DawRunOutcome::Quit => break,
                    DawRunOutcome::Restart => {
                        self.save_notepad_and_session_state();
                        return Ok(TuiExitReason::RestartApp);
                    }
                }
                continue;
            }
            sync_mouse_capture(
                &mut cleanup.mouse_capture_enabled,
                self.uses_mouse_capture(),
            )?;
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
            if self.active_screen == PrimaryScreen::Keyboard {
                self.pump_keyboard_periodic();
            }
            if self.active_screen == PrimaryScreen::GridSequencer && self.pump_grid_sequencer_step()
            {
                // コード進行データが更新されたので、アナウンス表示のあと再起動する。
                self.finish_grid_sequencer();
                self.save_notepad_and_session_state();
                return Ok(TuiExitReason::RestartApp);
            }
            if self.active_screen == PrimaryScreen::LoopBrowser {
                self.pump_loop_browser_step();
            }
            self.pump_notepad_sound_check_guide();
            clear_terminal_for_new_screen(&mut terminal, &mut rendered_screen, self.active_screen)?;
            let terminal_draw_started = std::time::Instant::now();
            terminal.draw(|f| self.draw(f))?;
            let terminal_draw_elapsed = terminal_draw_started.elapsed();
            if self.active_screen == PrimaryScreen::LoopBrowser {
                if let Some(metrics) = self.loop_browser.state.last_render_metrics.take() {
                    super::loop_browser::performance::log_render(metrics, terminal_draw_elapsed);
                }
            }
            if self.loop_browser.state.starting {
                self.complete_loop_browser_startup();
                continue;
            }
            // notepad 画面を実際に表示している間だけ起動時キャッシュを温める。
            // keyboard / loop browser / DAW で起動した場合は、notepad へ切り替わるまで走らせない。
            self.prime_notepad_startup_cache_if_needed(
                started_in_notepad_mode && self.cfg.autoplay_on_startup,
            );

            if event::poll(std::time::Duration::from_millis(50))? {
                let input = event::read()?;
                if let Event::Mouse(mouse) = input {
                    if self.active_screen == PrimaryScreen::GridSequencer
                        && !self.screen_switch_menu.is_open()
                    {
                        let size = terminal.backend().size()?;
                        self.handle_grid_sequencer_mouse_event(
                            mouse,
                            Rect::new(0, 0, size.width, size.height),
                        );
                    }
                    continue;
                }
                if matches!(input, Event::FocusLost) {
                    if self.active_screen == PrimaryScreen::GridSequencer {
                        self.grid_sequencer.cancel_mouse_gesture();
                    }
                    continue;
                }
                if let Event::Key(key) = input {
                    use crossterm::event::KeyEventKind;
                    if self.screen_switch_menu.is_open() {
                        if self.active_screen == PrimaryScreen::Keyboard
                            && key.kind == KeyEventKind::Release
                        {
                            let _ = self.handle_keyboard_key_event(key);
                        } else if key.kind == KeyEventKind::Press {
                            if let Some(target) = self.handle_screen_switch_menu_key(key) {
                                if target == crate::screen_switch::PrimaryScreen::Daw {
                                    sync_mouse_capture(&mut cleanup.mouse_capture_enabled, false)?;
                                    rendered_screen = None;
                                    match self.run_daw_screen(&mut terminal, false)? {
                                        DawRunOutcome::Continue => {}
                                        DawRunOutcome::Quit => break,
                                        DawRunOutcome::Restart => {
                                            self.save_notepad_and_session_state();
                                            return Ok(TuiExitReason::RestartApp);
                                        }
                                    }
                                } else {
                                    self.switch_to_primary_screen(target, None);
                                }
                            }
                        }
                        continue;
                    }
                    if key.kind == KeyEventKind::Press && self.try_open_screen_switch_menu(key) {
                        continue;
                    }
                    if self.active_screen == PrimaryScreen::Keyboard {
                        match self.handle_keyboard_key_event(key) {
                            KeyboardAction::Continue | KeyboardAction::ReturnToNotepad => {}
                            KeyboardAction::Quit => break,
                            KeyboardAction::LaunchDaw => {
                                rendered_screen = None;
                                match self.run_daw_screen(&mut terminal, false)? {
                                    DawRunOutcome::Continue => {}
                                    DawRunOutcome::Quit => break,
                                    DawRunOutcome::Restart => {
                                        self.save_notepad_and_session_state();
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
                        self.notepad.handle_ctrl_c(key);
                        continue;
                    }
                    if self.active_screen == PrimaryScreen::GridSequencer {
                        match self.handle_grid_sequencer_key_event(key) {
                            GridSequencerAction::Continue => {}
                            GridSequencerAction::Quit => {
                                self.finish_grid_sequencer();
                                break;
                            }
                            GridSequencerAction::RestartWithTrackCount(track_count) => {
                                debug_assert_eq!(self.grid_sequencer.track_count(), track_count);
                                self.save_notepad_and_session_state();
                                return Ok(TuiExitReason::RestartApp);
                            }
                        }
                        continue;
                    }
                    if self.active_screen == PrimaryScreen::LoopBrowser {
                        match self.loop_browser.state.handle_key_event(key) {
                            LoopBrowserAction::Continue => {}
                            LoopBrowserAction::Preview(path) => {
                                let trace_id =
                                    self.loop_browser.state.take_preview_trace().unwrap_or_else(
                                        super::loop_browser::performance::next_trace_id,
                                    );
                                self.preview_loop_file(path, trace_id);
                            }
                            LoopBrowserAction::Trigger { pad, path } => {
                                self.trigger_loop_pad(pad, path)
                            }
                            LoopBrowserAction::GridReplaced {
                                start_measure,
                                grid,
                                reason,
                            } => {
                                self.restart_loop_grid_at(grid, start_measure, reason);
                            }
                            LoopBrowserAction::GridRefresh { grid, reason } => {
                                self.update_loop_grid(grid, reason)
                            }
                            LoopBrowserAction::GridPreload {
                                grid,
                                token,
                                mode,
                                reason,
                            } => self.preload_loop_grid(grid, token, mode, reason),
                            LoopBrowserAction::TrackLayoutChanged {
                                start_measure,
                                grid,
                                track_volumes_db,
                                solo_tracks,
                            } => self.replace_loop_track_layout(
                                grid,
                                start_measure,
                                track_volumes_db,
                                solo_tracks,
                            ),
                            LoopBrowserAction::TrackVolumeChanged { track, volume_db } => {
                                self.update_loop_track_volume(track, volume_db)
                            }
                            LoopBrowserAction::TrackSoloChanged { solo_tracks } => {
                                self.update_loop_track_solo(solo_tracks)
                            }
                            LoopBrowserAction::SetPlaybackPaused {
                                paused,
                                start_measure,
                            } => self.set_loop_playback_paused(paused, start_measure),
                            LoopBrowserAction::BpmChanged { mode, grid } => {
                                self.set_loop_bpm_mode(mode, grid)
                            }
                            LoopBrowserAction::Quit => {
                                self.stop_loop_browser();
                                break;
                            }
                        }
                        continue;
                    }
                    match self.notepad.handle_key_event(key) {
                        NormalAction::Continue => {}
                        NormalAction::Quit => break,
                        NormalAction::LaunchDaw => {
                            rendered_screen = None;
                            match self.run_daw_screen(&mut terminal, false)? {
                                DawRunOutcome::Continue => {}
                                DawRunOutcome::Quit => break,
                                DawRunOutcome::Restart => {
                                    self.save_notepad_and_session_state();
                                    return Ok(TuiExitReason::RestartApp);
                                }
                            }
                        }
                        NormalAction::LaunchKeyboard => self.start_keyboard_from_notepad(),
                        NormalAction::LaunchLoopBrowser => self.begin_loop_browser_startup(),
                        NormalAction::EditConfig => {
                            let session = self.playback_session.begin();
                            self.playback_session
                                .set_play_state_if_current(session, PlayState::Idle);
                            match crate::config_editor::edit_config_toml(&mut terminal) {
                                Ok(()) => {
                                    self.save_notepad_and_session_state();
                                    return Ok(TuiExitReason::RestartApp);
                                }
                                Err(error) => {
                                    self.playback_session.set_play_state(PlayState::Err(format!(
                                        "config 編集に失敗しました: {error}"
                                    )));
                                }
                            }
                        }
                    }
                }
            }
        }

        // 終了前にセッション状態を保存する（端末クリーンアップの成否に関わらず実行）。
        // 保存失敗はベストエフォートとして無視する（終了処理のため通知手段がない）。
        self.save_notepad_and_session_state();
        Ok(TuiExitReason::Quit)
    }
}
