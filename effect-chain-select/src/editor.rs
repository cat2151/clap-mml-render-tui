//! chain 一覧の編集状態とキー処理。

use std::cell::Cell;

use cmrt_core::AudioEffectCatalog;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::Value;

use crate::{clamped_index, stage_is_bypassed, stage_with_bypass, EffectAddState, PAGE_STEP};

/// chain 一覧のキーが host に求めること。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainKeyAction {
    None,
    /// 編集中の chain（[`EffectChainEditor::chain`]）を鳴らしてほしい。
    Preview,
    /// 追加（`a`、`None`）・差し替え（`r`、その段）の overlay を開いてほしい。
    /// preset が無いときに開かない判断は host がする。
    OpenAdd {
        replace_target: Option<usize>,
    },
    /// 編集した chain を書き戻して閉じてほしい。
    Commit,
    /// 編集を捨てて閉じてほしい。
    Close,
    Help,
}

/// 編集中の chain の写し。書き戻すまで host の chain には触れない。
#[derive(Default)]
pub struct EffectChainEditor {
    /// 配列の順 = 信号の順。catalog に無い要素も、表示（[`crate::stage_label`]）と削除はできる。
    pub chain: Vec<Value>,
    pub cursor: usize,
    /// chain 一覧の表示先頭。描画側が上下 30% の余白の規則で更新する。
    pub scroll_offset: Cell<usize>,
    /// `dd` の 1 打目を受けた。
    pub pending_delete: bool,
    pub add: EffectAddState,
}

impl EffectChainEditor {
    pub fn open(chain: Vec<Value>) -> Self {
        Self {
            chain,
            ..Self::default()
        }
    }

    pub fn move_cursor(&mut self, delta: isize) {
        self.cursor = clamped_index(self.cursor, delta, self.chain.len());
    }

    pub fn push_stage(&mut self, stage: Value) {
        self.chain.push(stage);
        self.cursor = self.chain.len() - 1;
    }

    /// `index` 段を `stage` に差し替え、カーソルをその段へ置く。範囲外なら何もしない。
    pub fn replace_stage(&mut self, index: usize, stage: Value) {
        if let Some(slot) = self.chain.get_mut(index) {
            *slot = stage;
            self.cursor = index;
        }
    }

    pub fn delete_at_cursor(&mut self) -> bool {
        if self.cursor >= self.chain.len() {
            return false;
        }
        self.chain.remove(self.cursor);
        self.cursor = self.cursor.min(self.chain.len().saturating_sub(1));
        true
    }

    /// カーソル段の bypass を toggle する。chain が空なら何もせず `false` を返す。
    pub fn toggle_bypass_at_cursor(&mut self) -> bool {
        let Some(stage) = self.chain.get(self.cursor) else {
            return false;
        };
        let next = stage_with_bypass(stage, !stage_is_bypassed(stage));
        self.chain[self.cursor] = next;
        true
    }

    /// カーソル段を `delta` 段だけ動かす（swap）。カーソルも付いていく。
    /// 端を越える移動は何もせず `false` を返す。
    pub fn move_stage(&mut self, delta: isize) -> bool {
        let len = self.chain.len();
        if len == 0 {
            return false;
        }
        let Some(target) = self.cursor.checked_add_signed(delta) else {
            return false;
        };
        if target >= len {
            return false;
        }
        self.chain.swap(self.cursor, target);
        self.cursor = target;
        true
    }

    /// 追加・差し替えの overlay の状態を catalog から組み直す。
    pub fn open_add(&mut self, catalog: &AudioEffectCatalog, replace_target: Option<usize>) {
        self.add = EffectAddState::open(catalog);
        self.add.replace_target = replace_target;
    }

    /// chain 一覧のキー処理。chain が変わる操作（`dd`・`b`・`Alt+↑↓`）の直後は
    /// [`ChainKeyAction::Preview`] を返す。`j`/`k` は chain を変えないので鳴らし直させない。
    pub fn handle_chain_key(&mut self, key: KeyEvent) -> ChainKeyAction {
        if key.code == KeyCode::Char('d') {
            if self.pending_delete {
                self.pending_delete = false;
                if self.delete_at_cursor() {
                    return ChainKeyAction::Preview;
                }
            } else {
                self.pending_delete = true;
            }
            return ChainKeyAction::None;
        }
        self.pending_delete = false;

        if key.modifiers.contains(KeyModifiers::ALT) {
            let moved = match key.code {
                KeyCode::Down => self.move_stage(1),
                KeyCode::Up => self.move_stage(-1),
                _ => false,
            };
            return preview_if(moved);
        }

        match key.code {
            KeyCode::Esc => ChainKeyAction::Close,
            KeyCode::Char('?') => ChainKeyAction::Help,
            KeyCode::Char('j') | KeyCode::Down => self.moved(1),
            KeyCode::Char('k') | KeyCode::Up => self.moved(-1),
            KeyCode::PageDown => self.moved(PAGE_STEP),
            KeyCode::PageUp => self.moved(-PAGE_STEP),
            KeyCode::Home => self.moved(isize::MIN),
            KeyCode::End => self.moved(isize::MAX),
            KeyCode::Char('b') => preview_if(self.toggle_bypass_at_cursor()),
            KeyCode::Char(' ') => ChainKeyAction::Preview,
            KeyCode::Char('a') => ChainKeyAction::OpenAdd {
                replace_target: None,
            },
            KeyCode::Char('r') if self.cursor < self.chain.len() => ChainKeyAction::OpenAdd {
                replace_target: Some(self.cursor),
            },
            KeyCode::Enter => ChainKeyAction::Commit,
            _ => ChainKeyAction::None,
        }
    }

    fn moved(&mut self, delta: isize) -> ChainKeyAction {
        self.move_cursor(delta);
        ChainKeyAction::None
    }

    /// 追加 overlay の list `index` の preset を編集中 chain へ入れた chain。候補の段に `bypass` を付けられる。
    /// 差し替え（`replace_target`）ならその段を置き換え、そうでなければ末尾に足す。
    /// list が空、または index が範囲外なら `None`。
    pub fn candidate_chain(
        &self,
        catalog: Option<&AudioEffectCatalog>,
        index: usize,
        bypass: bool,
    ) -> Option<Vec<Value>> {
        let stage = self.add.preset_stage(catalog, index)?;
        let stage = stage_with_bypass(&stage, bypass);
        let mut chain = self.chain.clone();
        match self.add.replace_target {
            Some(target) if target < chain.len() => chain[target] = stage,
            _ => chain.push(stage),
        }
        Some(chain)
    }
}

fn preview_if(changed: bool) -> ChainKeyAction {
    if changed {
        ChainKeyAction::Preview
    } else {
        ChainKeyAction::None
    }
}

#[cfg(test)]
mod tests;
