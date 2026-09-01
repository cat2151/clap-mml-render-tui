use anyhow::Result;
use crossterm::{
    cursor::SetCursorStyle,
    event::{DisableMouseCapture, EnableMouseCapture, PopKeyboardEnhancementFlags},
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};

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
