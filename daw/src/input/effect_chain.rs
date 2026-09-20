//! EFFECT CHAIN overlay（`x`）のキー処理。
//!
//! overlay は init セルの chain の写しを編集し、Enter で 1 回だけ書き戻す。書き戻しは
//! 音色変更と同じ [`DawApp::commit_insert_cell`] を通すので、依存セルの cache 無効化と
//! 再 render もそこに任せる（chain は cache key（MML の hash）に入っている）。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::{
    messages::effect_chain as message,
    mml::build_cell_mml_from_data,
    overlays::{
        clamped_index, DawEffectAddState, DawEffectChainOverlayState, EffectAddPane, PAGE_STEP,
    },
    DawApp, DawMode, DawPlayState, FIRST_PLAYABLE_TRACK,
};

mod preview;

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
        if self.effect_plugins.catalog().is_none() {
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
        self.effect_plugins
            .catalog()
            .map_or(0, |catalog| catalog.presets().len())
    }

    /// chain 一覧のキー処理。chain が変わる操作（`dd`・`b`・`Alt+↑↓`）の直後は
    /// 自動で preview する。`j`/`k` は chain を変えないので鳴らし直さない。
    pub(crate) fn handle_effect_chain(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('d') {
            let state = &mut self.overlays.effect_chain;
            if state.pending_delete {
                state.pending_delete = false;
                if state.delete_at_cursor() {
                    self.preview_editing_effect_chain();
                }
            } else {
                state.pending_delete = true;
            }
            return;
        }
        self.overlays.effect_chain.pending_delete = false;

        if key.modifiers.contains(KeyModifiers::ALT) {
            let moved = match key.code {
                KeyCode::Down => self.overlays.effect_chain.move_stage(1),
                KeyCode::Up => self.overlays.effect_chain.move_stage(-1),
                _ => false,
            };
            if moved {
                self.preview_editing_effect_chain();
            }
            return;
        }

        match key.code {
            KeyCode::Esc => self.mode = DawMode::Normal,
            KeyCode::Char('j') | KeyCode::Down => self.overlays.effect_chain.move_cursor(1),
            KeyCode::Char('k') | KeyCode::Up => self.overlays.effect_chain.move_cursor(-1),
            KeyCode::PageDown => self.overlays.effect_chain.move_cursor(PAGE_STEP),
            KeyCode::PageUp => self.overlays.effect_chain.move_cursor(-PAGE_STEP),
            KeyCode::Home => self.overlays.effect_chain.move_cursor(isize::MIN),
            KeyCode::End => self.overlays.effect_chain.move_cursor(isize::MAX),
            KeyCode::Char('b') => {
                if self.overlays.effect_chain.toggle_bypass_at_cursor() {
                    self.preview_editing_effect_chain();
                }
            }
            KeyCode::Char(' ') => self.preview_editing_effect_chain(),
            KeyCode::Char('a') => {
                if self.effect_preset_count() == 0 {
                    self.append_log_line(message::NO_PRESETS);
                    return;
                }
                if let Some(catalog) = self.effect_plugins.catalog() {
                    self.overlays.effect_chain.add = DawEffectAddState::open(catalog);
                }
                self.mode = DawMode::EffectChainAdd;
                self.preview_effect_chain_add_candidate(None);
            }
            KeyCode::Enter => self.commit_effect_chain(),
            _ => {}
        }
    }

    /// `x` → `a` の role/list 2 pane。role を動かした後は list を絞り直しカーソルを 0 へ戻す
    /// （[`DawEffectAddState::rebuild_list`]）。list が 0 件の `Enter` は何もしない。
    /// `/` は focus がどちらのペインでも list の絞り込みを開始する。
    ///
    /// 候補（list カーソルの preset）が変わる操作のあとは自動で preview する。
    /// `h`/`l` は候補を変えないので鳴らし直さない。
    pub(crate) fn handle_effect_chain_add(&mut self, key: KeyEvent) {
        if self.overlays.effect_chain.add.filter_active {
            self.handle_effect_chain_add_filter_input(key);
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.mode = DawMode::EffectChain;
                return;
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.overlays.effect_chain.add.focus = EffectAddPane::Roles;
                return;
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.overlays.effect_chain.add.focus = EffectAddPane::List;
                return;
            }
            KeyCode::Char('/') => {
                self.overlays.effect_chain.add.begin_filter();
                return;
            }
            KeyCode::Char(' ') => {
                self.preview_effect_chain_add_candidate(None);
                return;
            }
            _ => {}
        }

        let delta = match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(1),
            KeyCode::Char('k') | KeyCode::Up => Some(-1),
            KeyCode::PageDown => Some(PAGE_STEP),
            KeyCode::PageUp => Some(-PAGE_STEP),
            KeyCode::Home => Some(isize::MIN),
            KeyCode::End => Some(isize::MAX),
            _ => None,
        };
        if let Some(delta) = delta {
            let add = &self.overlays.effect_chain.add;
            let candidate_before = add.list.get(add.list_cursor).copied();
            match self.overlays.effect_chain.add.focus {
                EffectAddPane::Roles => {
                    let len = self.overlays.effect_chain.add.roles.len();
                    self.overlays.effect_chain.add.role_cursor =
                        clamped_index(self.overlays.effect_chain.add.role_cursor, delta, len);
                    if let Some(catalog) = self.effect_plugins.catalog() {
                        self.overlays.effect_chain.add.rebuild_list(catalog);
                    }
                }
                EffectAddPane::List => {
                    let len = self.overlays.effect_chain.add.list.len();
                    self.overlays.effect_chain.add.list_cursor =
                        clamped_index(self.overlays.effect_chain.add.list_cursor, delta, len);
                }
            }
            let add = &self.overlays.effect_chain.add;
            if add.list.get(add.list_cursor).copied() != candidate_before {
                self.preview_effect_chain_add_candidate(Some(delta));
            }
            return;
        }

        if key.code == KeyCode::Enter {
            let add = &self.overlays.effect_chain.add;
            let Some(preset_index) = add.list.get(add.list_cursor).copied() else {
                return;
            };
            let stage = self
                .effect_plugins
                .catalog()
                .and_then(|catalog| catalog.presets().get(preset_index))
                .map(cmrt_core::AudioEffectPreset::json_element);
            if let Some(stage) = stage {
                self.overlays.effect_chain.push_stage(stage);
            }
            self.mode = DawMode::EffectChain;
        }
    }

    /// list の絞り込み編集中のキー処理。`Esc` で編集前の query へ戻す、`Enter` で確定、
    /// それ以外は textarea へ渡して list を更新する（他のキーは selector に渡さない）。
    fn handle_effect_chain_add_filter_input(&mut self, key: KeyEvent) {
        cmrt_tui_core::text_input::sync_single_line_textarea(
            &mut self.overlays.effect_chain.add.query_textarea,
            &self.overlays.effect_chain.add.query,
        );
        match key.code {
            KeyCode::Esc => {
                if let Some(catalog) = self.effect_plugins.catalog() {
                    self.overlays.effect_chain.add.cancel_filter(catalog);
                } else {
                    self.overlays.effect_chain.add.filter_active = false;
                }
            }
            KeyCode::Enter => {
                self.overlays.effect_chain.add.filter_active = false;
            }
            _ => {
                if cmrt_tui_core::text_input::apply_key_event_to_textarea(
                    &mut self.overlays.effect_chain.add.query_textarea,
                    key,
                ) {
                    self.overlays.effect_chain.add.query =
                        cmrt_tui_core::text_input::textarea_value(
                            &self.overlays.effect_chain.add.query_textarea,
                        );
                    let add = &self.overlays.effect_chain.add;
                    let candidate_before = add.list.get(add.list_cursor).copied();
                    if let Some(catalog) = self.effect_plugins.catalog() {
                        self.overlays.effect_chain.add.rebuild_list(catalog);
                    }
                    let add = &self.overlays.effect_chain.add;
                    if add.list.get(add.list_cursor).copied() != candidate_before {
                        self.preview_effect_chain_add_candidate(None);
                    }
                }
            }
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
