//! `y` で今の状態を共有コマンドにして clipboard へ書き、中央に通知を出す。

use crate::{share_command, KeyboardScreen};

impl KeyboardScreen<'_> {
    /// 今の復帰時情報から作った共有コマンドを clipboard へ書き、通知を開く。
    pub(crate) fn copy_share_command(&mut self) {
        let command = share_command(&self.session_state());
        cmrt_tui_core::clipboard::set_text(command.clone());
        self.share_notice = Some(command);
    }

    /// 通知中なら、コピーした共有コマンド。
    pub fn share_notice(&self) -> Option<&str> {
        self.share_notice.as_deref()
    }
}

#[cfg(test)]
mod tests;
