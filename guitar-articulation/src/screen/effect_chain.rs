//! effect chain overlay（`x`）。METAL-GTX の後ろに掛ける chain（ギターアンプなど）を選ぶ。
//!
//! overlay は chain の写しを編集し、Enter で画面の chain へ書き戻す（Esc は捨てる）。
//! 試聴は常に Articulated を、試聴したい chain で鳴らす。

use cmrt_core::AudioEffectCatalog;
use cmrt_effect_chain_select::{messages, AddKeyAction, ChainKeyAction, EffectChainEditor};
use crossterm::event::KeyEvent;
use serde_json::Value;

use super::{GuitarArticulationAction, GuitarArticulationScreen, Take};

/// 開いている overlay。`adding` は追加・差し替えの 3 pane を出しているか。
#[derive(Default)]
pub(super) struct EffectOverlay {
    pub(super) editor: EffectChainEditor,
    pub(super) adding: bool,
}

impl GuitarArticulationScreen {
    /// 確定済みの chain。dry の間も持ったまま。
    pub fn effect_chain(&self) -> &[Value] {
        &self.effect_chain
    }

    /// `b` / `space` の演奏に掛ける chain。dry の間は空。
    pub fn sounding_effect_chain(&self) -> &[Value] {
        if self.effect_dry {
            &[]
        } else {
            &self.effect_chain
        }
    }

    /// effect を掛けずに鳴らしているか（`w`）。
    pub fn effect_dry(&self) -> bool {
        self.effect_dry
    }

    pub fn effect_overlay_open(&self) -> bool {
        self.effect_overlay.is_some()
    }

    /// 編集中の chain と、追加・差し替えの 3 pane を出しているか。overlay が閉じていれば `None`。
    pub fn effect_overlay(&self) -> Option<(&EffectChainEditor, bool)> {
        self.effect_overlay
            .as_ref()
            .map(|overlay| (&overlay.editor, overlay.adding))
    }

    /// 追加・差し替えの list を絞り込む入力欄にキーが入る状態か。
    pub fn effect_filter_active(&self) -> bool {
        self.effect_overlay
            .as_ref()
            .is_some_and(|overlay| overlay.adding && overlay.editor.add.filter_active)
    }

    /// effect の catalog。未走査なら走査を待つので、chain を持っていないときの描画からは呼ばない。
    pub fn effect_catalog(&self) -> Option<&AudioEffectCatalog> {
        self.effect_plugins.catalog()
    }

    pub(super) fn open_effect_overlay(&mut self) -> GuitarArticulationAction {
        if self.effect_catalog().is_none() {
            self.error = Some(messages::NOT_AVAILABLE_ON_THIS_BACKEND.to_string());
            return GuitarArticulationAction::Continue;
        }
        self.effect_overlay = Some(EffectOverlay {
            editor: EffectChainEditor::open(self.effect_chain.clone()),
            adding: false,
        });
        GuitarArticulationAction::Continue
    }

    pub(super) fn handle_effect_key(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        let Some(mut overlay) = self.effect_overlay.take() else {
            return GuitarArticulationAction::Continue;
        };
        self.error = None;
        let catalog = self.effect_plugins.catalog();
        let action = if overlay.adding {
            match overlay.editor.handle_add_key(catalog, key) {
                AddKeyAction::None => GuitarArticulationAction::Continue,
                AddKeyAction::Back | AddKeyAction::Committed => {
                    overlay.adding = false;
                    GuitarArticulationAction::Continue
                }
                AddKeyAction::Help => {
                    self.help_open = true;
                    GuitarArticulationAction::Continue
                }
                AddKeyAction::Preview { .. } => {
                    let cursor = overlay.editor.add.list_cursor;
                    self.preview(overlay.editor.candidate_chain(catalog, cursor, false))
                }
                AddKeyAction::PreviewBypassed => {
                    let cursor = overlay.editor.add.list_cursor;
                    self.preview(overlay.editor.candidate_chain(catalog, cursor, true))
                }
            }
        } else {
            match overlay.editor.handle_chain_key(key) {
                ChainKeyAction::None => GuitarArticulationAction::Continue,
                ChainKeyAction::Preview => self.preview(Some(overlay.editor.chain.clone())),
                ChainKeyAction::OpenAdd { replace_target } => match catalog {
                    Some(catalog) if !catalog.presets().is_empty() => {
                        overlay.editor.open_add(catalog, replace_target);
                        overlay.adding = true;
                        let cursor = overlay.editor.add.list_cursor;
                        self.preview(overlay.editor.candidate_chain(Some(catalog), cursor, false))
                    }
                    _ => {
                        self.error = Some(messages::NO_PRESETS.to_string());
                        GuitarArticulationAction::Continue
                    }
                },
                ChainKeyAction::Commit => {
                    // 選んだ chain を聴くために確定したので、dry なら wet へ戻す。
                    self.effect_chain = overlay.editor.chain;
                    self.effect_dry = false;
                    self.record_history();
                    return self.play(Take::Converted);
                }
                ChainKeyAction::Close => return GuitarArticulationAction::Continue,
                ChainKeyAction::Help => {
                    self.help_open = true;
                    GuitarArticulationAction::Continue
                }
            }
        };
        self.effect_overlay = Some(overlay);
        action
    }

    /// Articulated をその chain で鳴らしてほしいと返す。MML が無ければ鳴らすものが無いので理由を出す。
    fn preview(&mut self, chain: Option<Vec<Value>>) -> GuitarArticulationAction {
        let Some(chain) = chain else {
            return GuitarArticulationAction::Continue;
        };
        if self.plain.is_empty() {
            self.error = Some("i で MML を入力してください".to_string());
            return GuitarArticulationAction::Continue;
        }
        GuitarArticulationAction::PreviewEffectChain(chain)
    }
}

#[cfg(test)]
mod tests;
