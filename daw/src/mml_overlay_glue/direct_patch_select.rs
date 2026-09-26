//! NORMAL の `t` で、MML overlay の音色 selector（`Ctrl+T` と同じもの）を直接開く。
//!
//! MML 入力欄は selector の土台として裏で開くだけで、selector を確定 / 取消したら
//! overlay ごと閉じて NORMAL へ戻る。確定した音色は `Ctrl+T` と同じ経路で
//! その track の init セルへ書き戻される。

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};

use super::super::{DawApp, DawMode, CHORD_TRACK, FIRST_PLAYABLE_TRACK};
use cmrt_mml_overlay::MmlOverlaySyntax;

use super::INIT_MEASURE;

impl DawApp {
    /// カーソル track の音色 selector を開く。開けたら true。
    pub(crate) fn open_direct_patch_select(&mut self) -> bool {
        if self.mode != DawMode::Normal {
            return false;
        }
        if self.editor.cursor_track < FIRST_PLAYABLE_TRACK
            && self.editor.cursor_track != CHORD_TRACK
        {
            self.append_log_line("音色選択は演奏トラックでのみ使用できます".to_string());
            return false;
        }
        let Some(target_track) = self.mml_overlay_target_track() else {
            self.append_log_line(
                "chord preview の演奏 track がありません。演奏 track の init に \"generate from chord track\" を設定してください",
            );
            return false;
        };
        let mut context = self.mml_overlay_context();
        // 試聴行は、音色を選ぶ track がその meas で DAW 上鳴らす MML。chord 行でも
        // chord 表記ではなく生成先 track の生成結果にする（Grid import の recipe は
        // chord の試聴文脈では再現できない）。init 列は音色 JSON なので空行。
        context.syntax = MmlOverlaySyntax::Mml;
        context.initial_text = if self.editor.cursor_measure == INIT_MEASURE {
            String::new()
        } else {
            crate::mml::cell_preview_line(
                &self.editor.data,
                target_track,
                self.editor.cursor_measure,
            )
        };
        self.open_mml_overlay_with(context);
        self.mml_overlay_patch_select_only = true;
        self.mml_overlay.request_patch_select();
        // 一覧が Error / 空なら selector は開かない。入力欄だけを残さず閉じる。
        if !self.mml_overlay.is_patch_select_open()
            && !self.mml_overlay.is_waiting_for_patch_catalog()
        {
            self.append_log_line("音色一覧を開けませんでした（一覧が空か読み込み失敗）");
            self.dismiss_direct_patch_select();
            return true;
        }
        let action = self.mml_overlay.preview_current_patch(Instant::now());
        self.apply_mml_overlay_action(action);
        true
    }

    /// selector を閉じた時点で overlay ごと閉じる。
    ///
    /// 一覧の Loading 中は selector がまだ無いので、`Esc` だけを受けて閉じ、
    /// 他のキーは入力欄へ通さない。
    pub(super) fn handle_direct_patch_select_key_event(&mut self, key: KeyEvent) {
        if !self.mml_overlay.is_patch_select_open() {
            if key.code == KeyCode::Esc {
                self.dismiss_direct_patch_select();
            }
            return;
        }
        self.forward_key_to_mml_overlay(key);
        if self.mode == DawMode::MmlOverlay && !self.mml_overlay.is_patch_select_open() {
            self.dismiss_direct_patch_select();
        }
    }

    fn dismiss_direct_patch_select(&mut self) {
        self.mml_overlay.dismiss();
        self.close_mml_overlay();
    }
}

#[cfg(test)]
pub(super) mod tests;
