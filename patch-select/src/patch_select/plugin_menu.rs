//! 音色 selector の `m` で開く plugin solo/mute overlay。中身は [`crate::plugin_menu`]。

use cmrt_tui_core::text_input;
use crossterm::event::KeyEvent;

use super::{PatchSelect, PatchSelectAction};
use crate::plugin_menu::{PluginMenu, PluginMenuKey};

impl PatchSelect<'_> {
    pub(crate) fn plugin_menu(&self) -> Option<&PluginMenu> {
        self.plugin_menu.as_ref()
    }

    /// 確定済みの絞り込み条件。
    pub(crate) fn committed_query(&self) -> &str {
        &self.committed_query
    }

    /// 確定済みの絞り込みで `slug` が solo / mute されているか。
    #[cfg(test)]
    pub(crate) fn plugin_mode(&self, slug: &str) -> Option<crate::plugin_menu::PluginMode> {
        crate::plugin_menu::plugin_mode(&self.committed_query, slug)
    }

    pub(super) fn open_plugin_menu(&mut self) -> PatchSelectAction {
        self.plugin_menu = Some(PluginMenu::new(&self.all));
        PatchSelectAction::Continue
    }

    /// menu に無いキーは menu を開いたまま無視する。
    pub(super) fn handle_plugin_menu_key(&mut self, key: KeyEvent) -> PatchSelectAction {
        let Some(menu) = &self.plugin_menu else {
            return PatchSelectAction::Continue;
        };
        match menu.handle_key(key, &self.committed_query) {
            PluginMenuKey::Ignored => PatchSelectAction::Continue,
            PluginMenuKey::Close => {
                self.plugin_menu = None;
                PatchSelectAction::Continue
            }
            PluginMenuKey::Chosen(condition) => {
                self.plugin_menu = None;
                self.committed_query = condition;
                self.query = text_input::new_single_line_textarea(&self.committed_query);
                self.refilter()
            }
        }
    }
}
