//! MML 入力オーバーレイと共有ランタイムの接続。
//!
//! オーバーレイ自体は `cmrt-mml-overlay` crate に閉じている。ここが持つのは
//! 「どの画面から開いてよいか」「開くときに何を止めるか」「鳴らす先はどこか」
//! 「開くときに何のスナップショットを渡すか」の 4 つだけ。

use std::time::Instant;

use crossterm::event::KeyEvent;

use super::mml_overlay::{
    is_mml_overlay_trigger, MmlOverlayAction, MmlOverlayContext, PatchCatalogSnapshot, PatchChange,
};
use super::{PatchLoadState, TuiApp};

impl TuiApp<'_> {
    /// Ctrl+P ならオーバーレイを開く。開いたら true。
    ///
    /// 開ける条件は画面切替メニューと同じにしてある。どちらも「いまの画面が
    /// モーダルな入力中でないこと」を求めるため。
    pub(in crate::tui) fn try_open_mml_overlay(&mut self, key: KeyEvent) -> bool {
        if !is_mml_overlay_trigger(key) || !self.can_open_screen_switch_menu() {
            return false;
        }
        // オーバーレイは keyboard 画面と同じ音源インスタンスを借りるので、
        // 先にいまの画面の演奏を止めて明け渡してもらう。
        self.stop_active_screen_playback();
        let context = self.mml_overlay_context();
        self.mml_overlay.open(context);
        if let Some(sender) = &self.mml_overlay_sender {
            let command_id = sender.prepare(self.mml_overlay.patch());
            self.mml_overlay.expect_sender_command(command_id);
        }
        true
    }

    /// 音色一覧の状態とフレーズ履歴を、開くたびに最新のスナップショットで渡す。
    /// 音色一覧が Loading なら、完了後に [`Self::pump_mml_overlay`] が差し替える。
    fn mml_overlay_context(&self) -> MmlOverlayContext {
        let patch_catalog = self.mml_overlay_patch_catalog_snapshot();
        let (history, favorites) = self.notepad.phrase_history();
        MmlOverlayContext {
            patch_catalog,
            history: history.to_vec(),
            favorites: favorites.to_vec(),
            catalog_notes: self.catalog_notes.clone(),
        }
    }

    fn mml_overlay_patch_catalog_snapshot(&self) -> PatchCatalogSnapshot {
        match &*self.patch_load_state.lock().unwrap() {
            PatchLoadState::Loading => PatchCatalogSnapshot::Loading,
            PatchLoadState::Ready(pairs) => PatchCatalogSnapshot::Ready(pairs.clone()),
            PatchLoadState::Err(error) => PatchCatalogSnapshot::Error(error.clone()),
        }
    }

    #[cfg(test)]
    pub(in crate::tui) fn loaded_patch_pairs(&self) -> Vec<(String, String)> {
        match &*self.patch_load_state.lock().unwrap() {
            PatchLoadState::Ready(pairs) => pairs.clone(),
            PatchLoadState::Loading | PatchLoadState::Err(_) => Vec::new(),
        }
    }

    /// オーバーレイが開いている間、キーはすべてオーバーレイが取る。
    pub(in crate::tui) fn handle_mml_overlay_key_event(&mut self, key: KeyEvent) {
        // loader 完了と Ctrl+T が同じ frame に来ても、古い Loading を見せない。
        self.sync_mml_overlay_patch_catalog();
        let action = self.mml_overlay.handle_key(key, Instant::now());
        self.apply_mml_overlay_action(action);
    }

    /// worker が実際に到達した一覧・loading・発音状態を表示へ反映する。毎フレーム呼ぶ。
    pub(in crate::tui) fn pump_mml_overlay(&mut self) {
        self.sync_mml_overlay_patch_catalog();
        if let Some(sender) = &self.mml_overlay_sender {
            self.mml_overlay.sync_sender_status(&sender.status());
        }
    }

    fn sync_mml_overlay_patch_catalog(&mut self) {
        if !self.mml_overlay.is_waiting_for_patch_catalog() {
            return;
        }
        let catalog = self.mml_overlay_patch_catalog_snapshot();
        self.mml_overlay.sync_patch_catalog(catalog);
    }

    /// オーバーレイの求めを sender へ流す。
    ///
    /// note off はここには出てこない。「鳴っているものを止める」は sender 側が
    /// 1 か所で持っていて、音を鳴らすコマンドはどれも停止込みの意味になっている。
    fn apply_mml_overlay_action(&mut self, action: MmlOverlayAction) {
        // 閉じるときだけ sender の外へ用がある（音源を借りていた画面へ返す）ので、
        // sender を借りる前に片づける。
        if action == MmlOverlayAction::Close {
            if let Some(sender) = &self.mml_overlay_sender {
                let command_id = sender.stop();
                self.mml_overlay.expect_sender_command(command_id);
            }
            // 借りていた音源を返す。開いたときに止めた演奏はここで戻る。
            self.resume_active_screen_playback();
            return;
        }
        let Some(sender) = &self.mml_overlay_sender else {
            return;
        };
        let command_id = match action {
            MmlOverlayAction::Continue | MmlOverlayAction::Close => None,
            MmlOverlayAction::Send(notes) => {
                let id = sender.send(self.mml_overlay.patch(), notes.messages, notes.duration);
                Some(id)
            }
            MmlOverlayAction::SetPatch { patch, notes } => Some(match notes {
                Some(notes) => sender.send(patch.as_deref(), notes.messages, notes.duration),
                None => sender.prepare(patch.as_deref()),
            }),
            MmlOverlayAction::PlayLine { patch, events } => {
                let patch = match &patch {
                    PatchChange::Keep => self.mml_overlay.patch(),
                    PatchChange::Switch(patch) => patch.as_deref(),
                };
                Some(sender.play_line(patch, events))
            }
        };
        if let Some(command_id) = command_id {
            self.mml_overlay.expect_sender_command(command_id);
        }
    }
}
