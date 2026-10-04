//! 右端の Effect pane（chain 一覧）と、`a` / `r` で開く追加・差し替え overlay。
//!
//! chain は画面に 1 つで、写しを持たない。pane で chain を変えたらすぐ鳴っている音へ掛け直す。
//! overlay の候補は試聴として掛け、`Esc` なら pane の chain へ掛け直す。

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use serde_json::Value;

use cmrt_effect_chain_select::{
    chain_json, messages, AddKeyAction, ChainKeyAction, EffectChainEditor,
};

use super::{state::note_for_key, KeyboardAction, KeyboardScreen, PatchPaneFocus};

#[derive(Default)]
pub struct KeyboardEffectPane {
    editor: EffectChainEditor,
    adding: bool,
    /// 鳴っている音へ最後に掛けた chain。試聴中は候補入り。
    sounding: Vec<Value>,
    /// `a` / `r` を受けたが選べる preset が無かった。
    no_presets: bool,
}

impl KeyboardEffectPane {
    pub fn restored(chain: Vec<Value>) -> Self {
        Self {
            sounding: chain.clone(),
            editor: EffectChainEditor::open(chain),
            ..Self::default()
        }
    }

    /// 確定済みの chain（保存する物）。
    pub fn chain(&self) -> &[Value] {
        &self.editor.chain
    }

    pub fn editor(&self) -> &EffectChainEditor {
        &self.editor
    }

    /// 追加・差し替え overlay を開いているか。
    pub fn is_adding(&self) -> bool {
        self.adding
    }

    /// overlay の list 絞り込み欄に打鍵している最中か。
    pub fn is_typing(&self) -> bool {
        self.adding && self.editor.add.filter_active
    }

    /// 鳴っている音へ最後に掛けた chain。
    pub fn sounding(&self) -> &[Value] {
        &self.sounding
    }

    /// chain が空のときに pane へ出す文言。`available` は effect を持つ経路か。
    pub fn empty_notice(&self, available: bool) -> &'static str {
        if !available {
            messages::NOT_AVAILABLE_ON_THIS_BACKEND
        } else if self.no_presets {
            messages::NO_PRESETS
        } else {
            messages::EMPTY_CHAIN
        }
    }

    pub(crate) fn move_cursor(&mut self, delta: isize) {
        self.editor.pending_delete = false;
        self.editor.move_cursor(delta);
    }

    pub(crate) fn clear_pending_delete(&mut self) {
        self.editor.pending_delete = false;
    }

    /// overlay を閉じ、試聴中の候補を捨てる。掛け直すべき chain（確定済みの chain）を返す。
    pub(crate) fn close_overlay(&mut self) -> Vec<Value> {
        self.adding = false;
        self.sounding = self.editor.chain.clone();
        self.sounding.clone()
    }
}

impl KeyboardScreen<'_> {
    pub fn effect_pane(&self) -> &KeyboardEffectPane {
        &self.effect
    }

    /// `chain` を鳴っている音へ掛ける。音色の準備がまだなら sender が覚え、次の準備に同梱する。
    pub(crate) fn sound_effect_chain(&mut self, chain: Vec<Value>) {
        if let Some(sender) = &self.midi_sender {
            sender.set_effect_chain(&chain_json(&chain));
        }
        self.effect.sounding = chain;
    }

    /// Effect pane に focus があるときのキー。pane が使ったら `Some`。
    /// 使わないキー（h/l・音符 `c e f g`・Release など）は通常の処理へ流す。
    pub(crate) fn handle_effect_pane_key(&mut self, key: KeyEvent) -> Option<KeyboardAction> {
        if self.state.patch_catalog.focus() != PatchPaneFocus::Effect
            || key.kind != KeyEventKind::Press
        {
            return None;
        }
        let used = match key.modifiers {
            KeyModifiers::NONE => matches!(key.code, KeyCode::Char('a' | 'b' | 'd' | 'r')),
            KeyModifiers::ALT => matches!(key.code, KeyCode::Up | KeyCode::Down),
            _ => false,
        };
        if !used {
            self.effect.editor.pending_delete = false;
            return None;
        }
        self.effect.no_presets = false;
        match self.effect.editor.handle_chain_key(key) {
            ChainKeyAction::Preview => {
                let chain = self.effect.editor.chain.clone();
                self.sound_effect_chain(chain);
            }
            ChainKeyAction::OpenAdd { replace_target } => self.open_effect_add(replace_target),
            ChainKeyAction::None
            | ChainKeyAction::Commit
            | ChainKeyAction::Close
            | ChainKeyAction::Help => {}
        }
        Some(KeyboardAction::Continue)
    }

    fn open_effect_add(&mut self, replace_target: Option<usize>) {
        let effect_plugins = self.effect_plugins().clone();
        let Some(catalog) = effect_plugins.catalog() else {
            return;
        };
        if catalog.presets().is_empty() {
            self.effect.no_presets = true;
            return;
        }
        self.effect.editor.open_add(catalog, replace_target);
        self.effect.adding = true;
        let cursor = self.effect.editor.add.list_cursor;
        if let Some(chain) = self
            .effect
            .editor
            .candidate_chain(Some(catalog), cursor, false)
        {
            self.sound_effect_chain(chain);
        }
    }

    /// 追加・差し替え overlay を開いている間のキー。Release は note off、overlay が使わない
    /// 音符キー（`b` 以外）は音符として鳴らす。
    pub(crate) fn handle_effect_add_key(&mut self, key: KeyEvent) -> KeyboardAction {
        if key.kind == KeyEventKind::Release {
            self.release_note_while_typing(key);
            return KeyboardAction::Continue;
        }
        let typing = self.effect.editor.add.filter_active;
        if !typing
            && key.modifiers == KeyModifiers::NONE
            && key.code != KeyCode::Char('b')
            && note_for_key(key.code).is_some()
        {
            return self.handle_note_key(key);
        }
        let effect_plugins = self.effect_plugins().clone();
        let catalog = effect_plugins.catalog();
        let action = self.effect.editor.handle_add_key(catalog, key);
        let cursor = self.effect.editor.add.list_cursor;
        let preview = match action {
            AddKeyAction::None | AddKeyAction::Help => None,
            AddKeyAction::Back | AddKeyAction::Committed => Some(self.effect.close_overlay()),
            AddKeyAction::Preview { .. } => {
                self.effect.editor.candidate_chain(catalog, cursor, false)
            }
            AddKeyAction::PreviewBypassed => {
                self.effect.editor.candidate_chain(catalog, cursor, true)
            }
        };
        if let Some(chain) = preview {
            self.sound_effect_chain(chain);
        }
        KeyboardAction::Continue
    }
}

#[cfg(test)]
pub(crate) mod tests;
