//! DAW モードのメインループ

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::{DawApp, DawExitReason, DawMode, DawNormalAction};

impl DawApp {
    /// TuiApp と同じ terminal を受け取って DAW モードを実行する。
    /// 終了時は `DawExitReason` を返す:
    ///   - `ReturnToTui` : d キーで TUI に戻る
    ///   - `QuitApp`     : q キーでアプリを終了する
    pub fn run_with_terminal(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<DawExitReason> {
        self.kick_all_pending();

        loop {
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
                        if self.mode == DawMode::Insert {
                            self.handle_insert(key);
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
                        DawMode::History => self.handle_history_overlay(key.code),
                    }
                }
            }
        }
    }
}
