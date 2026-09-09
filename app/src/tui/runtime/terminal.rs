//! 画面ごとに変わる端末モード（マウスキャプチャ・カーソル形状）の要求と適用、
//! および終了時の後始末。

use anyhow::Result;
use crossterm::{
    cursor::SetCursorStyle,
    event::{DisableMouseCapture, EnableMouseCapture, PopKeyboardEnhancementFlags},
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};

use crate::tui::{PrimaryScreen, TuiApp};

impl TuiApp<'_> {
    pub(crate) fn uses_mouse_capture(&self) -> bool {
        self.active_screen == PrimaryScreen::GridSequencer
    }

    /// 端末カーソルを textarea の位置に置く画面か。
    ///
    /// true のときだけ点滅する縦線カーソルにする（issue #334: 入力中に見えるカーソルは
    /// 入力欄の 1 つだけにする。list 側の bg 強調を落とすのは各画面の描画の役目）。
    pub(crate) fn uses_textarea_cursor(&self) -> bool {
        if self.mml_overlay.is_open() {
            return true;
        }
        match self.active_screen {
            PrimaryScreen::Keyboard => self.keyboard.mml_input.is_active(),
            // loop browser の textarea は loop tree の `/` 絞り込み入力欄だけ。
            PrimaryScreen::LoopBrowser => self.loop_browser.state.filter_input_active(),
            PrimaryScreen::GridSequencer => false,
            // chord chart の textarea は section の進行 / 名前を打つ `e` / `n` の欄だけ。
            PrimaryScreen::ChordChart => self.chord_chart.line_input_open(),
            PrimaryScreen::Notepad | PrimaryScreen::DailyDaw | PrimaryScreen::Daw => {
                self.notepad.uses_textarea_cursor()
            }
        }
    }
}

pub(super) struct TerminalCleanup {
    pub(super) raw_mode_enabled: bool,
    pub(super) alternate_screen_enabled: bool,
    pub(super) line_wrap_disabled: bool,
    pub(super) keyboard_enhancement_enabled: bool,
    pub(super) mouse_capture_enabled: bool,
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
        if self.line_wrap_disabled {
            let _ = super::line_wrap::enable(&mut std::io::stdout());
        }
        if self.alternate_screen_enabled {
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        }
        if self.raw_mode_enabled {
            let _ = disable_raw_mode();
        }
    }
}

pub(super) fn sync_mouse_capture(enabled: &mut bool, requested: bool) -> Result<()> {
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

/// カーソル形状を端末へ反映する。`current` が `None` なら（＝起動直後で端末の
/// 状態が分からないので）必ず書き、以降は変化したときだけ書く。
pub(super) fn sync_cursor_shape(current: &mut Option<bool>, requested: bool) -> Result<()> {
    if *current == Some(requested) {
        return Ok(());
    }
    execute!(
        std::io::stdout(),
        if requested {
            SetCursorStyle::BlinkingBar
        } else {
            SetCursorStyle::DefaultUserShape
        }
    )?;
    *current = Some(requested);
    Ok(())
}
