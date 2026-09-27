//! DAW のログ生成側と表示側で共有する文言・書式。

use std::time::SystemTime;

pub(crate) const STARTUP_LOG_PREFIX: &str = "--- DAW 起動 ";
pub(crate) const STARTUP_LOG_SUFFIX: &str = " ---";
pub(crate) const SHIFT_SPACE_LOG_LINE: &str = "Shift+Space: カーソル小節から演奏 / 停止";

pub(crate) fn startup_log_line(now: SystemTime) -> String {
    format!(
        "{STARTUP_LOG_PREFIX}{}{STARTUP_LOG_SUFFIX}",
        cmrt_tui_core::logging::format_jst_timestamp(now)
    )
}
