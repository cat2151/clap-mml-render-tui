//! EFFECT CHAIN overlay（`x`）のキー処理。
//!
//! overlay は init セルの chain の写しを編集し、Enter で 1 回だけ書き戻す。書き戻しは
//! 音色変更と同じ [`DawApp::commit_insert_cell`] を通すので、依存セルの cache 無効化と
//! 再 render もそこに任せる（chain は cache key（MML の hash）に入っている）。

use crossterm::event::KeyCode;

use super::super::{
    messages::effect_chain as message,
    overlays::{clamped_index, DawEffectChainOverlayState},
    DawApp, DawMode, FIRST_PLAYABLE_TRACK,
};

/// init 列。effect chain はここにだけ入る。
const INIT_MEASURE: usize = 0;

impl DawApp {
    /// `x`: cursor track の EFFECT CHAIN overlay を開く。演奏 track 以外では開かない。
    pub(crate) fn start_effect_chain_overlay(&mut self) {
        let track = self.editor.cursor_track;
        if track < FIRST_PLAYABLE_TRACK {
            self.append_log_line(message::PLAYABLE_TRACK_ONLY);
            return;
        }
        if self.plugin_entries.effects().catalog().is_none() {
            self.append_log_line(message::NOT_AVAILABLE_ON_THIS_BACKEND);
            return;
        }
        let init_cell = self.editor.data[track][INIT_MEASURE].clone();
        let instrument = self.track_patch_name(track).unwrap_or_default();
        let chain = crate::mml::effect_chain::init_cell_effect_chain(&init_cell);
        self.overlays.effect_chain = DawEffectChainOverlayState::open(track, instrument, chain);
        self.mode = DawMode::EffectChain;
    }

    pub(crate) fn effect_preset_count(&self) -> usize {
        self.plugin_entries
            .effects()
            .catalog()
            .map_or(0, |catalog| catalog.presets().len())
    }

    pub(crate) fn handle_effect_chain(&mut self, key: KeyCode) {
        if key == KeyCode::Char('d') {
            let state = &mut self.overlays.effect_chain;
            if state.pending_delete {
                state.pending_delete = false;
                state.delete_at_cursor();
            } else {
                state.pending_delete = true;
            }
            return;
        }
        self.overlays.effect_chain.pending_delete = false;

        match key {
            KeyCode::Esc => self.mode = DawMode::Normal,
            KeyCode::Char('j') | KeyCode::Down => self.overlays.effect_chain.move_cursor(1),
            KeyCode::Char('k') | KeyCode::Up => self.overlays.effect_chain.move_cursor(-1),
            KeyCode::Char('a') => {
                if self.effect_preset_count() == 0 {
                    self.append_log_line(message::NO_PRESETS);
                    return;
                }
                self.overlays.effect_chain.add_cursor = 0;
                self.mode = DawMode::EffectChainAdd;
            }
            KeyCode::Enter => self.commit_effect_chain(),
            _ => {}
        }
    }

    pub(crate) fn handle_effect_chain_add(&mut self, key: KeyCode) {
        let preset_count = self.effect_preset_count();
        let state = &mut self.overlays.effect_chain;
        match key {
            KeyCode::Esc => self.mode = DawMode::EffectChain,
            KeyCode::Char('j') | KeyCode::Down => {
                state.add_cursor = clamped_index(state.add_cursor, 1, preset_count);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                state.add_cursor = clamped_index(state.add_cursor, -1, preset_count);
            }
            KeyCode::Enter => {
                let stage = self
                    .plugin_entries
                    .effects()
                    .catalog()
                    .and_then(|catalog| {
                        catalog.presets().get(self.overlays.effect_chain.add_cursor)
                    })
                    .map(cmrt_core::AudioEffectPreset::json_element);
                if let Some(stage) = stage {
                    self.overlays.effect_chain.push_stage(stage);
                }
                self.mode = DawMode::EffectChain;
            }
            _ => {}
        }
    }

    /// Enter: 編集した chain を init セルへ書き戻して閉じる。変わっていなければ書かない。
    fn commit_effect_chain(&mut self) {
        let track = self.overlays.effect_chain.track;
        let next_init = crate::mml::effect_chain::init_cell_with_effect_chain(
            &self.editor.data[track][INIT_MEASURE],
            &self.overlays.effect_chain.chain,
        );
        if self.commit_insert_cell(track, INIT_MEASURE, &next_init) {
            self.save();
            self.sync_playback_mml_state();
            self.append_log_line(format!(
                "effect chain: {} を {} 段にした",
                crate::tracks::track_label(track),
                self.overlays.effect_chain.chain.len()
            ));
        }
        self.mode = DawMode::Normal;
    }
}
