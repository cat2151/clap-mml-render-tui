//! MML 入力オーバーレイと共有ランタイムの接続。
//!
//! オーバーレイ自体は `cmrt-mml-overlay` crate に閉じている。ここが持つのは
//! 「どの画面から開いてよいか」「開くときに何を止めるか」「鳴らす先はどこか」
//! 「開くときに何のスナップショットを渡すか」の 4 つだけ。

use std::time::Instant;

use crossterm::event::KeyEvent;

use super::mml_overlay::{
    host_patch_catalog, is_mml_overlay_trigger, HostPatchCatalog, MmlOverlayAction,
    MmlOverlayContext, MmlOverlayInputMode, PatchChange,
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
        let HostPatchCatalog {
            catalog: patch_catalog,
            patch_role_index,
            load_measurements,
        } = self.mml_overlay_patch_catalog_snapshot();
        let (history, favorites) = self.notepad.phrase_history();
        let catalog_notes = match &*self.patch_load_state.lock().unwrap() {
            PatchLoadState::Ready(snapshot) if !snapshot.catalog_notes().is_empty() => {
                snapshot.catalog_notes().to_vec()
            }
            PatchLoadState::Ready(_) => self.catalog_notes.clone(),
            PatchLoadState::Loading | PatchLoadState::Err(_) => Vec::new(),
        };
        MmlOverlayContext {
            // app からの Ctrl+P は従来どおり複数行・空の入力欄で開く。
            // 1 行モードは DAW が明示的に指定したときだけ。
            input_mode: MmlOverlayInputMode::MultiLine,
            initial_text: String::new(),
            patch_catalog,
            patch_role_index,
            load_measurements,
            history: history.to_vec(),
            favorites: favorites.to_vec(),
            patch_filter_presets: crate::history::load_mml_patch_filter_presets(),
            catalog_notes,
        }
    }

    /// 一覧・Role 索引・load 計測は DAW と同じ 1 実装（`cmrt_mml_overlay::host_patch_catalog`）で
    /// 作る。`Loading` / `Err` のときに何を渡すかが画面ごとに食い違わないようにするため。
    fn mml_overlay_patch_catalog_snapshot(&self) -> HostPatchCatalog {
        host_patch_catalog(&self.patch_load_state.lock().unwrap())
    }

    #[cfg(test)]
    pub(in crate::tui) fn loaded_patch_pairs(&self) -> Vec<(String, String)> {
        match &*self.patch_load_state.lock().unwrap() {
            PatchLoadState::Ready(snapshot) => snapshot.pairs().to_vec(),
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
        let HostPatchCatalog {
            catalog,
            patch_role_index,
            load_measurements,
        } = self.mml_overlay_patch_catalog_snapshot();
        self.mml_overlay
            .sync_patch_catalog(catalog, patch_role_index, load_measurements);
    }

    /// オーバーレイの求めを sender へ流す。
    ///
    /// note off はここには出てこない。「鳴っているものを止める」は sender 側が
    /// 1 か所で持っていて、音を鳴らすコマンドはどれも停止込みの意味になっている。
    fn apply_mml_overlay_action(&mut self, action: MmlOverlayAction) {
        let action = match action {
            MmlOverlayAction::SavePatchFilterPresets { presets, preview } => {
                if let Err(error) = crate::history::save_mml_patch_filter_presets(&presets) {
                    crate::logging::global_log_sink(&format!(
                        "mml-overlay: action=patch-filter-preset event=save result=error detail={error:?}"
                    ));
                } else if let PatchLoadState::Ready(snapshot) =
                    &mut *self.patch_load_state.lock().unwrap()
                {
                    std::sync::Arc::make_mut(snapshot).rebuild_patch_roles(&presets);
                }
                if let (Some(sender), Some((patch, notes))) = (&self.mml_overlay_sender, preview) {
                    let command_id = match notes {
                        Some(notes) => sender.send(Some(&patch), notes.messages, notes.duration),
                        None => sender.prepare(Some(&patch)),
                    };
                    self.mml_overlay.expect_sender_command(command_id);
                }
                return;
            }
            action => action,
        };
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
            // Commit は 1 行モードでしか返らない。app 側は複数行モードでしか
            // 開かないのでここへは来ない（来ても sender へ用は無い）。
            MmlOverlayAction::Continue
            | MmlOverlayAction::Close
            | MmlOverlayAction::Commit { .. }
            | MmlOverlayAction::SavePatchFilterPresets { .. } => None,
            MmlOverlayAction::Send(notes) => {
                let id = sender.send(self.mml_overlay.patch(), notes.messages, notes.duration);
                Some(id)
            }
            MmlOverlayAction::SetPatch { patch, notes } => Some(match notes {
                Some(notes) => sender.send(patch.as_deref(), notes.messages, notes.duration),
                None => sender.prepare(patch.as_deref()),
            }),
            MmlOverlayAction::PlayLine { patch, program } => {
                let patch = match &patch {
                    PatchChange::Keep => self.mml_overlay.patch(),
                    PatchChange::Switch(patch) => patch.as_deref(),
                };
                Some(sender.play_line(patch, program))
            }
        };
        if let Some(command_id) = command_id {
            self.mml_overlay.expect_sender_command(command_id);
        }
    }
}

#[cfg(test)]
mod tests;
