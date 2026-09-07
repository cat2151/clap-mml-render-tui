//! 選択中のノードへ付ける印（お気に入り・カテゴリ・WAV pad）の付け外し。
//!
//! いずれも `metadata` へ書いて保存し、可視ノードを組み直す点が共通。
//! カテゴリだけは選択用の overlay を挟むので、そのキー処理もここに置く。

use super::*;

impl LoopBrowser {
    pub(crate) fn handle_category_overlay_key(&mut self, key: KeyCode) -> LoopBrowserAction {
        match key {
            KeyCode::Esc => self.category_overlay = None,
            KeyCode::Char(key) => {
                let key = key.to_ascii_lowercase();
                if let Some(category) = self
                    .category_keys
                    .iter()
                    .find(|(candidate, _)| *candidate == key)
                    .map(|(_, category)| category.clone())
                {
                    if self.assign_selected_category(category) {
                        return LoopBrowserAction::GridRefresh {
                            grid: self.playback_grid(),
                            reason: LoopGridChange::Category,
                        };
                    }
                }
            }
            _ => {}
        }
        LoopBrowserAction::Continue
    }

    pub(crate) fn selected_target_dir(&self) -> Option<LoopDirId> {
        let node = self.visible.get(self.cursor)?;
        let root_path = &self.roots.get(node.key.root)?.0;
        let mut components = node.key.components.clone();
        if node.is_wav {
            components.pop();
        }
        let relative = components.iter().collect::<PathBuf>();
        Some(LoopDirId::new(root_path, &relative))
    }

    pub(crate) fn toggle_selected_favorite(&mut self) {
        let Some(dir) = self.selected_target_dir() else {
            return;
        };
        if !self.metadata.writable {
            return;
        }
        let selected_path = self.visible.get(self.cursor).map(|node| node.path.clone());
        let Some(added) = self.metadata.try_mutate(
            |metadata| metadata.toggle_favorite(&dir),
            |path, metadata| metadata.save_to(path),
            "お気に入りを保存できません",
        ) else {
            return;
        };
        self.rebuild_favorite_wav_keys();
        if !added {
            self.notice = Some(LoopBrowserNotice {
                text: "お気に入りdirを解除しました".to_string(),
                expires_at: Instant::now() + REMOVED_NOTICE_DURATION,
            });
        }
        self.rebuild_visible_for_path(selected_path.as_deref());
    }

    pub(crate) fn toggle_favorites_only(&mut self) {
        let selected_path = self.visible.get(self.cursor).map(|node| node.path.clone());
        self.favorites_only = !self.favorites_only;
        self.category_overlay = None;
        self.rebuild_visible_for_path(selected_path.as_deref());
    }

    pub(crate) fn open_category_overlay(&mut self) {
        if self.category_keys.is_empty() || !self.metadata.writable {
            return;
        }
        self.category_overlay = self.selected_target_dir();
    }

    pub(crate) fn assign_selected_category(&mut self, category: String) -> bool {
        let Some(dir) = self.category_overlay.take() else {
            return false;
        };
        let selected_path = self.visible.get(self.cursor).map(|node| node.path.clone());
        if self
            .metadata
            .try_mutate(
                |metadata| metadata.toggle_category(&dir, &category),
                |path, metadata| metadata.save_to(path),
                "カテゴリを保存できません",
            )
            .is_none()
        {
            return false;
        }
        self.rebuild_wav_categories();
        self.rebuild_visible_for_path(selected_path.as_deref());
        true
    }

    pub(crate) fn selected_wav_id(&self) -> Option<LoopWavId> {
        let node = self.visible.get(self.cursor)?;
        if !node.is_wav {
            return None;
        }
        let root = &self.roots.get(node.key.root)?.0;
        let relative = node.key.components.iter().collect::<PathBuf>();
        Some(LoopWavId::new(root, &relative))
    }

    pub(crate) fn toggle_selected_pad(&mut self, pad: char) -> LoopBrowserAction {
        let Some(wav) = self.selected_wav_id() else {
            return LoopBrowserAction::Continue;
        };
        if !self.metadata.writable {
            return LoopBrowserAction::Continue;
        }
        let Some(assigned) = self.metadata.try_mutate(
            |metadata| metadata.toggle_pad(pad, &wav),
            |path, metadata| metadata.save_to(path),
            "WAV padを保存できません",
        ) else {
            return LoopBrowserAction::Continue;
        };
        if !assigned {
            self.notice = Some(LoopBrowserNotice {
                text: format!("WAV pad {} を解除しました", pad.to_ascii_uppercase()),
                expires_at: Instant::now() + REMOVED_NOTICE_DURATION,
            });
        } else {
            self.notice = None;
        }
        LoopBrowserAction::Continue
    }
}
