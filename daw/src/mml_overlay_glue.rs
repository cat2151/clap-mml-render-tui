//! DAW から開く MML 入力オーバーレイの配線。
//!
//! オーバーレイ本体は `cmrt-mml-overlay` crate に閉じている。ここが持つのは
//! 「どのセルで開いてよいか」「開くときに何を止めるか」「どの音色で鳴らすか」
//! 「求められた発音を誰へ流すか」の 4 つだけ。
//!
//! DAW では **1 行モード**で開く（`Enter` が改行ではなく確定）。従来の
//! インライン INSERT と同じ手触りにするため。複数行モード（app の `Ctrl+P`）の
//! 挙動には一切触らない。
//!
//! **init 列（meas 0）では開かない。** init セルの中身は音色 JSON なので、
//! 1 行 MML として上書きさせると音色指定が壊れる。init 列は従来どおり `i` で編集する。

use std::sync::Arc;
use std::time::Instant;

use crossterm::event::KeyEvent;

use cmrt_mml_overlay::{
    host_patch_catalog, is_mml_overlay_trigger, HostPatchCatalog, MmlOverlayAction,
    MmlOverlayContext, MmlOverlayInputMode, PatchChange,
};
use cmrt_tui_core::patch_load::PatchLoadState;

use super::input::track_patch::PatchUpdateReason;
use super::{DawApp, DawMode};

/// init 列。ここでは overlay を開かない。
const INIT_MEASURE: usize = 0;

impl DawApp {
    /// `Ctrl+P` ならオーバーレイを開く。開いたら true。
    pub(crate) fn try_open_mml_overlay(&mut self, key: KeyEvent) -> bool {
        if !is_mml_overlay_trigger(key) {
            return false;
        }
        self.open_mml_overlay()
    }

    /// `i` の入口。MML 入力はオーバーレイで行う。
    ///
    /// init 列（meas 0）だけは中身が音色 JSON なので、従来のインライン INSERT に落ちる。
    /// 「どの列がオーバーレイの対象外か」を知っているのはこのモジュールだけにしたいので、
    /// 分岐は呼び出し側（`input/normal.rs`）ではなくここへ置く。
    pub(crate) fn open_mml_overlay_or_insert(&mut self) {
        if self.editor.cursor_measure == INIT_MEASURE || !self.open_mml_overlay() {
            self.start_insert();
        }
    }

    /// カーソルのあるセルの MML を、1 行モードのオーバーレイで開く。開けたら true。
    ///
    /// `i` からも同じ入口を使えるように、キー判定とは分けてある。
    pub(crate) fn open_mml_overlay(&mut self) -> bool {
        if self.mode != DawMode::Normal {
            return false;
        }
        if self.editor.cursor_measure == INIT_MEASURE {
            self.append_log_line(
                "init 列は音色セルです。MML 入力は meas1 以降で開きます".to_string(),
            );
            return false;
        }
        // オーバーレイは keyboard 画面と同じ音源 instance を借りる。
        // 先に DAW の演奏を止めて明け渡す。閉じても自動では再開しない。
        self.stop_play();
        let context = self.mml_overlay_context();
        self.mml_overlay.open(context);
        // そのセルが DAW で実際に鳴る音色を、オーバーレイの音色として渡す。
        // 渡さないと別の音色で鳴り、書いた音と grid の音が食い違う。
        let patch = self.current_track_patch_name();
        self.mml_overlay.set_restored_patch(patch);
        if let Some(sender) = &self.mml_overlay_sender {
            let command_id = sender.prepare(self.mml_overlay.patch());
            self.mml_overlay.expect_sender_command(command_id);
        }
        self.mode = DawMode::MmlOverlay;
        true
    }

    /// 開くたびに、音色一覧とフレーズ履歴の最新スナップショットを渡す。
    fn mml_overlay_context(&self) -> MmlOverlayContext {
        let HostPatchCatalog {
            catalog,
            patch_role_index,
            load_measurements,
        } = host_patch_catalog(&self.patch_load.lock().unwrap());
        let (history, favorites) = self.mml_overlay_phrase_history();
        MmlOverlayContext {
            // DAW は 1 行モード。`Enter` は改行ではなく確定。
            input_mode: MmlOverlayInputMode::SingleLine,
            initial_text: self.editor.data[self.editor.cursor_track][self.editor.cursor_measure]
                .clone(),
            patch_catalog: catalog,
            patch_role_index,
            load_measurements,
            history,
            favorites,
            patch_filter_presets: cmrt_history::load_mml_patch_filter_presets(),
            catalog_notes: self.mml_overlay_catalog_notes(),
        }
    }

    /// `Ctrl+O` のフレーズ履歴。DAW の history overlay と同じ選び方にする
    /// （その track に音色があればその音色の履歴、無ければ notepad の履歴）。
    fn mml_overlay_phrase_history(&self) -> (Vec<String>, Vec<String>) {
        let patch_history = self
            .current_track_patch_name()
            .and_then(|patch_name| self.patch_phrase_store.patches.get(&patch_name))
            .map(|state| (state.history.clone(), state.favorites.clone()))
            .filter(|(history, favorites)| !history.is_empty() || !favorites.is_empty());
        patch_history.unwrap_or_else(|| {
            (
                self.patch_phrase_store.notepad.history.clone(),
                self.patch_phrase_store.notepad.favorites.clone(),
            )
        })
    }

    /// 設定不足でカタログから外れたプラグインの案内。一覧に**出てこない**ものの話なので、
    /// 音色一覧をいくら見ても気づけない。
    fn mml_overlay_catalog_notes(&self) -> Vec<String> {
        self.catalog_snapshot()
            .map(|snapshot| snapshot.catalog_notes().to_vec())
            .unwrap_or_default()
    }

    /// 開いている間、キーはすべてオーバーレイが取る。
    pub(crate) fn handle_mml_overlay_key_event(&mut self, key: KeyEvent) {
        // loader 完了と Ctrl+T が同じ frame に来ても、古い Loading を見せない。
        self.sync_mml_overlay_patch_catalog();
        // 音色一覧をカーソルで流しているだけ（preview）では `patch()` は変わらない。
        // 変わるのは `Ctrl+T` / `Ctrl+O` の確定と、ホストが入れたときだけなので、
        // **前後の比較 1 か所**で「確定した音色」だけを init セルへ反映できる。
        let patch_before = self.mml_overlay.patch().map(str::to_string);
        let action = self.mml_overlay.handle_key(key, Instant::now());
        self.apply_mml_overlay_action(action);
        self.reflect_mml_overlay_patch_change(patch_before);
    }

    /// オーバーレイで音色が確定したら、その track の init セルへ書き戻す。
    ///
    /// DAW にとって「オーバーレイの音色」はその track の init meas の音色そのもの。
    /// preview で暴発しないよう、**変化したときだけ**書く。
    fn reflect_mml_overlay_patch_change(&mut self, patch_before: Option<String>) {
        let patch_after = self.mml_overlay.patch().map(str::to_string);
        if patch_after == patch_before {
            return;
        }
        let Some(patch_name) = patch_after else {
            return;
        };
        let track = self.editor.cursor_track;
        let patch_filter_query = self.track_patch_filter_query(track);
        // track 0（Tempo 行）は `apply_patch_name_to_track_init` 側で弾かれる。
        self.apply_patch_name_to_track_init(
            track,
            &patch_name,
            patch_filter_query.as_deref(),
            PatchUpdateReason::MmlOverlay,
        );
    }

    /// 1 行モードの確定。書き戻しの後、`Enter` なら次の meas の入力欄を開き直す。
    ///
    /// 確定しても preview は鳴らさない。打鍵で既に鳴っているうえ、音源が別
    /// （オーバーレイは play server の instance、preview はレンダリング済み PCM）なので
    /// 二重再生になる。
    fn commit_mml_overlay_line(&mut self, line: &str, close: bool) {
        let track = self.editor.cursor_track;
        let measure = self.editor.cursor_measure;
        if self.commit_insert_cell(track, measure, line) {
            self.save();
            // hot reload: 次の再生ループから新しい MML を反映する。
            self.sync_playback_mml_state();
        }
        if close {
            self.close_mml_overlay();
            return;
        }
        // 既存のインライン INSERT と同じ流れ。最終 meas ではそのセルのまま開き直す。
        if self.editor.cursor_measure < self.editor.measures {
            self.editor.cursor_measure += 1;
            self.update_ab_repeat_follow_end_with_cursor();
        }
        // 音色は `open()` が引き継ぐので、prepare を積み直す必要は無い。
        let context = self.mml_overlay_context();
        self.mml_overlay.open(context);
    }

    /// 音色フィルタのプリセット追加を保存し、role の索引を作り直す。
    ///
    /// 作り直さないと、init 列の `role:音色名` 表示がプリセット追加に追従しない。
    fn save_mml_overlay_patch_filter_presets(&mut self, presets: &[(String, String)]) {
        if let Err(error) = cmrt_history::save_mml_patch_filter_presets(presets) {
            self.append_log_line(format!(
                "mml-overlay: action=patch-filter-preset event=save result=error detail={error:?}"
            ));
            return;
        }
        if let PatchLoadState::Ready(snapshot) = &mut *self.patch_load.lock().unwrap() {
            Arc::make_mut(snapshot).rebuild_patch_roles(presets);
        }
    }

    /// worker が実際に到達した一覧・発音状態を表示へ反映する。毎フレーム呼ぶ。
    pub(crate) fn pump_mml_overlay(&mut self) {
        if !self.mml_overlay.is_open() {
            return;
        }
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
        } = host_patch_catalog(&self.patch_load.lock().unwrap());
        self.mml_overlay
            .sync_patch_catalog(catalog, patch_role_index, load_measurements);
    }

    /// オーバーレイの求めを sender へ流す。
    ///
    /// note off はここには出てこない。「鳴っているものを止める」は sender 側が
    /// 1 か所で持っていて、音を鳴らすコマンドはどれも停止込みの意味になっている。
    fn apply_mml_overlay_action(&mut self, action: MmlOverlayAction) {
        // sender を借りる前に片づけるもの。閉じる指示は 2 通りある
        // （`Esc` は 1 行モードでは `Commit { close: true }` で返り、overlay 側は既に
        // 閉じている。`Close` だけを見ていると閉じられない）。
        let action = match action {
            MmlOverlayAction::Commit { line, close } => {
                self.commit_mml_overlay_line(&line, close);
                return;
            }
            MmlOverlayAction::Close => {
                self.close_mml_overlay();
                return;
            }
            MmlOverlayAction::SavePatchFilterPresets { presets, preview } => {
                self.save_mml_overlay_patch_filter_presets(&presets);
                let Some((patch, notes)) = preview else {
                    return;
                };
                if let Some(sender) = &self.mml_overlay_sender {
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
        let Some(sender) = &self.mml_overlay_sender else {
            return;
        };
        let command_id = match action {
            // 上で返しているのでここへは来ない。
            MmlOverlayAction::Continue
            | MmlOverlayAction::Close
            | MmlOverlayAction::Commit { .. }
            | MmlOverlayAction::SavePatchFilterPresets { .. } => None,
            MmlOverlayAction::Send(notes) => {
                Some(sender.send(self.mml_overlay.patch(), notes.messages, notes.duration))
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

    /// 借りていた音源を返して NORMAL へ戻る。開いたときに止めた演奏は再開しない。
    fn close_mml_overlay(&mut self) {
        if let Some(sender) = &self.mml_overlay_sender {
            let command_id = sender.stop();
            self.mml_overlay.expect_sender_command(command_id);
        }
        self.mode = DawMode::Normal;
    }
}

#[cfg(test)]
mod tests;
