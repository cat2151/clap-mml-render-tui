//! `M` で開く plugin solo/mute overlay。
//!
//! 中身とキーの意味は patch selector と同じ `cmrt_patch_select::plugin_menu`。選んだ結果は
//! Patches pane の絞り込み条件へ plugin term として足し引きする。

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use cmrt_patch_select::plugin_menu::{PluginMenu, PluginMenuKey};

use super::{KeyboardAction, KeyboardContext, KeyboardScreen};

/// overlay を開くキー。端末によって Shift が修飾に載るので、どちらも受ける。
pub(crate) fn is_plugin_menu_key(key: KeyEvent) -> bool {
    key.kind == KeyEventKind::Press
        && match key.modifiers {
            KeyModifiers::NONE => key.code == KeyCode::Char('M'),
            KeyModifiers::SHIFT => matches!(key.code, KeyCode::Char('m' | 'M')),
            _ => false,
        }
}

impl KeyboardScreen<'_> {
    /// 開いている plugin solo/mute overlay。
    pub fn plugin_menu(&self) -> Option<&PluginMenu> {
        self.plugin_menu.as_ref()
    }

    pub(crate) fn open_plugin_menu(&mut self, ctx: &KeyboardContext<'_>) {
        self.sync_patch_catalog(ctx);
        self.plugin_menu = Some(PluginMenu::new(self.state.patch_catalog.entries()));
    }

    /// Press は overlay へ、Release は note off として通す。menu に無いキーは開いたまま無視する。
    pub(crate) fn handle_plugin_menu_key(
        &mut self,
        key: KeyEvent,
        ctx: &KeyboardContext<'_>,
    ) -> KeyboardAction {
        match key.kind {
            KeyEventKind::Release => {
                self.release_note_while_typing(key);
                return KeyboardAction::Continue;
            }
            // 押しっぱなしで同じ plugin を solo と解除の間で往復させない。
            KeyEventKind::Repeat => return KeyboardAction::Continue,
            KeyEventKind::Press => {}
        }
        let Some(menu) = &self.plugin_menu else {
            return KeyboardAction::Continue;
        };
        match menu.handle_key(key, self.state.patch_catalog.filter()) {
            PluginMenuKey::Ignored => {}
            PluginMenuKey::Close => self.plugin_menu = None,
            PluginMenuKey::Chosen(condition) => {
                self.plugin_menu = None;
                self.apply_patch_filter(&condition, ctx);
                self.state.patch_catalog.measure_patch_list();
            }
        }
        KeyboardAction::Continue
    }
}

#[cfg(test)]
mod tests;
