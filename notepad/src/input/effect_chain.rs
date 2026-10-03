//! EFFECT CHAIN overlay（`x`）のキー処理。
//!
//! overlay はカーソル行の行頭 JSON にある chain の写しを編集し、Enter で行へ書き戻す
//! （ESC は捨てる）。`;` で区切った全パートの行頭 JSON に同じ chain を書く。
//! 試聴は通常の再生経路（[`NotepadScreen::play_mml`]）で、編集中の chain を載せた行を鳴らす。
//! auto reverb の印は触らず、chain の key だけを書き換える。

use cmrt_core::EFFECT_CHAIN_JSON_KEY;
use cmrt_effect_chain_select::{messages, AddKeyAction, ChainKeyAction, EffectChainEditor};
use crossterm::event::KeyEvent;
use serde_json::Value;

use super::patch_select::line_with_patch_json;
use crate::{Mode, NotepadScreen, PlayState, PATCH_NAME_JSON_NOT_FOUND};

impl<'a> NotepadScreen<'a> {
    /// `x`: カーソル行の chain で overlay を開く。音色の行頭 JSON が無い行と、
    /// effect の catalog を持たない経路では開かない。
    pub(super) fn open_effect_chain_overlay(&mut self) {
        if self.current_line_patch_name().is_none() {
            self.set_effect_chain_error(PATCH_NAME_JSON_NOT_FOUND);
            return;
        }
        if self.effect_plugins.catalog().is_none() {
            self.set_effect_chain_error(messages::NOT_AVAILABLE_ON_THIS_BACKEND);
            return;
        }
        self.effect_chain = EffectChainEditor::open(self.current_line_effect_chain());
        self.mode = Mode::EffectChain;
    }

    /// chain 一覧のキー処理。chain が変わる操作の直後は自動で試聴する。
    pub(super) fn handle_effect_chain(&mut self, key: KeyEvent) {
        match self.effect_chain.handle_chain_key(key) {
            ChainKeyAction::None => {}
            ChainKeyAction::Preview => {
                let chain = self.effect_chain.chain.clone();
                self.preview_effect_chain(Some(chain));
            }
            ChainKeyAction::OpenAdd { replace_target } => {
                self.open_effect_chain_add(replace_target)
            }
            ChainKeyAction::Commit => self.commit_effect_chain(),
            ChainKeyAction::Close => self.mode = Mode::Normal,
            ChainKeyAction::Help => self.enter_help(),
        }
    }

    /// 追加・差し替えの 3 pane を開き、list カーソルの候補を試聴する。
    /// `replace_target` は `a` なら `None`、`r` ならカーソル段。
    fn open_effect_chain_add(&mut self, replace_target: Option<usize>) {
        let Some(catalog) = self
            .effect_plugins
            .catalog()
            .filter(|catalog| !catalog.presets().is_empty())
        else {
            self.set_effect_chain_error(messages::NO_PRESETS);
            return;
        };
        self.effect_chain.open_add(catalog, replace_target);
        self.mode = Mode::EffectChainAdd;
        self.preview_effect_chain_add_candidate(false);
    }

    /// 追加・差し替えの 3 pane のキー処理。候補が変わる操作のあとは自動で試聴する。
    pub(super) fn handle_effect_chain_add(&mut self, key: KeyEvent) {
        let action = self
            .effect_chain
            .handle_add_key(self.effect_plugins.catalog(), key);
        match action {
            AddKeyAction::None => {}
            AddKeyAction::Back | AddKeyAction::Committed => self.mode = Mode::EffectChain,
            AddKeyAction::Help => self.enter_help(),
            AddKeyAction::Preview { .. } => self.preview_effect_chain_add_candidate(false),
            AddKeyAction::PreviewBypassed => self.preview_effect_chain_add_candidate(true),
        }
    }

    /// list カーソルの候補を編集中 chain へ入れて試聴する。`bypass` なら候補の段だけ bypass する。
    fn preview_effect_chain_add_candidate(&mut self, bypass: bool) {
        let chain = self.effect_chain.candidate_chain(
            self.effect_plugins.catalog(),
            self.effect_chain.add.list_cursor,
            bypass,
        );
        self.preview_effect_chain(chain);
    }

    /// `chain` を載せたカーソル行を鳴らす。行は書き換えない。
    fn preview_effect_chain(&mut self, chain: Option<Vec<Value>>) {
        let Some(mml) = chain.and_then(|chain| self.current_line_with_effect_chain(&chain)) else {
            return;
        };
        let mml = mml.trim().to_string();
        if !mml.is_empty() {
            self.play_mml(mml);
        }
    }

    /// Enter: 編集した chain をカーソル行へ書き戻して閉じ、その行を鳴らす。
    /// chain が変わっていなければ行は書き換えない。
    fn commit_effect_chain(&mut self) {
        self.mode = Mode::Normal;
        let chain = std::mem::take(&mut self.effect_chain.chain);
        if chain != self.current_line_effect_chain() {
            if let Some(line) = self.current_line_with_effect_chain(&chain) {
                self.editor.lines[self.editor.cursor] = line;
            }
        }
        self.play_current_line();
    }

    /// カーソル行の行頭 JSON の chain。key が無い・配列でないときは空。
    fn current_line_effect_chain(&self) -> Vec<Value> {
        self.current_line_effect_keys()
            .get(EFFECT_CHAIN_JSON_KEY)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    /// カーソル行の全パートの行頭 JSON の chain を `chain` にした行。空の chain は key ごと消す。
    /// 音色名・filter 語・auto reverb の印・MML 本体はそのまま。音色の行頭 JSON が無ければ `None`。
    fn current_line_with_effect_chain(&self, chain: &[Value]) -> Option<String> {
        let patch_name = self.current_line_patch_name()?;
        let filter_query = self.current_line_patch_filter_query();
        let mut effects = self.current_line_effect_keys();
        if chain.is_empty() {
            effects.remove(EFFECT_CHAIN_JSON_KEY);
        } else {
            effects.insert(
                EFFECT_CHAIN_JSON_KEY.to_string(),
                Value::Array(chain.to_vec()),
            );
        }
        let json = Self::build_patch_json_with_filter_query(
            &patch_name,
            filter_query.as_deref(),
            &effects,
        );
        Some(line_with_patch_json(
            self.editor.lines.get(self.editor.cursor)?,
            &json,
        ))
    }

    fn set_effect_chain_error(&self, message: &str) {
        *self.playback.session.play_state().lock().unwrap() = PlayState::Err(message.to_string());
    }
}

#[cfg(test)]
mod tests;
