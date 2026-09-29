//! EFFECT CHAIN overlay（`x`）のキー処理。
//!
//! overlay は init セルの chain の写しを編集し、Enter で 1 回だけ書き戻す。書き戻しは
//! 音色変更と同じ [`DawApp::commit_insert_cell`] を通すので、依存セルの cache 無効化と
//! 再 render もそこに任せる（chain は cache key（MML の hash）に入っている）。

use cmrt_effect_chain_select::{messages as select_message, AddKeyAction, ChainKeyAction};
use crossterm::event::KeyEvent;

use super::super::{
    messages::effect_chain as message, mml::build_cell_mml_from_data,
    overlays::DawEffectChainOverlayState, DawApp, DawMode, DawPlayState, FIRST_PLAYABLE_TRACK,
};

mod preview;

/// init 列。effect chain はここにだけ入る。
const INIT_MEASURE: usize = 0;

impl DawApp {
    /// `x`: cursor track の EFFECT CHAIN overlay を開く。演奏 track 以外では開かない。
    /// 演奏中なら止める（演奏中の preview は鳴らさないため）。
    pub(crate) fn start_effect_chain_overlay(&mut self) {
        let track = self.editor.cursor_track;
        if track < FIRST_PLAYABLE_TRACK {
            self.append_log_line(message::PLAYABLE_TRACK_ONLY);
            return;
        }
        if self.effect_plugins.catalog().is_none() {
            self.append_log_line(select_message::NOT_AVAILABLE_ON_THIS_BACKEND);
            return;
        }
        if *self.playback.play_state.lock().unwrap() == DawPlayState::Playing {
            self.stop_play();
        }
        let init_cell = self.editor.data[track][INIT_MEASURE].clone();
        let instrument = self.track_patch_name(track).unwrap_or_default();
        let chain = crate::mml::effect_chain::init_cell_effect_chain(&init_cell);
        self.overlays.effect_chain = DawEffectChainOverlayState::open(track, instrument, chain);
        self.mode = DawMode::EffectChain;
    }

    pub(crate) fn effect_preset_count(&self) -> usize {
        self.effect_plugins
            .catalog()
            .map_or(0, |catalog| catalog.presets().len())
    }

    /// chain 一覧のキー処理。chain が変わる操作の直後は自動で preview する。
    pub(crate) fn handle_effect_chain(&mut self, key: KeyEvent) {
        match self.overlays.effect_chain.editor.handle_chain_key(key) {
            ChainKeyAction::None => {}
            ChainKeyAction::Preview => self.preview_editing_effect_chain(),
            ChainKeyAction::OpenAdd { replace_target } => {
                self.open_effect_chain_add(replace_target)
            }
            ChainKeyAction::Commit => self.commit_effect_chain(),
            ChainKeyAction::Close => self.mode = DawMode::Normal,
            ChainKeyAction::Help => self.enter_help(),
        }
    }

    /// 追加 overlay を開いて list カーソルの候補を preview する。`replace_target` は
    /// `EffectAddState::replace_target`（`a` は `None`、`r` はカーソル段）。
    fn open_effect_chain_add(&mut self, replace_target: Option<usize>) {
        if self.effect_preset_count() == 0 {
            self.append_log_line(select_message::NO_PRESETS);
            return;
        }
        if let Some(catalog) = self.effect_plugins.catalog() {
            self.overlays
                .effect_chain
                .editor
                .open_add(catalog, replace_target);
        }
        self.mode = DawMode::EffectChainAdd;
        self.preview_effect_chain_add_candidate(None);
    }

    /// `x` → `a`（または `r`）の category/kind/list 3 pane。候補が変わる操作のあとは自動で preview する。
    pub(crate) fn handle_effect_chain_add(&mut self, key: KeyEvent) {
        let action = self
            .overlays
            .effect_chain
            .editor
            .handle_add_key(self.effect_plugins.catalog(), key);
        match action {
            AddKeyAction::None => {}
            AddKeyAction::Back | AddKeyAction::Committed => self.mode = DawMode::EffectChain,
            AddKeyAction::Help => self.enter_help(),
            AddKeyAction::Preview { preferred_delta } => {
                self.preview_effect_chain_add_candidate(preferred_delta)
            }
            AddKeyAction::PreviewBypassed => self.preview_effect_chain_add_candidate_bypassed(),
        }
    }

    /// Enter: 編集した chain を init セルへ書き戻して閉じ、自動演奏を予約する。
    /// 変わっていなければ書かない。
    fn commit_effect_chain(&mut self) {
        let track = self.overlays.effect_chain.track;
        let next_init = crate::mml::effect_chain::init_cell_with_effect_chain(
            &self.editor.data[track][INIT_MEASURE],
            &self.overlays.effect_chain.editor.chain,
        );
        if self.commit_insert_cell(track, INIT_MEASURE, &next_init) {
            self.save();
            self.sync_playback_mml_state();
            self.append_log_line(format!(
                "effect chain: {} を {} 段にした",
                crate::tracks::track_label(track),
                self.overlays.effect_chain.editor.chain.len()
            ));
        }
        self.mode = DawMode::Normal;
        self.reserve_auto_play_after_render();
    }
}
