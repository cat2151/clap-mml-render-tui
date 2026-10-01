//! effect chain を編集する overlay（chain 一覧 + category/kind/list 3 pane の追加・差し替え）。
//!
//! 状態・キー処理・描画だけを持ち、音は鳴らさない。キー処理は「鳴らしてほしい chain」や
//! 「確定してほしい」を action で返し、試聴と書き戻しは host（DAW・Guitar Articulation）が行う。
//! chain の段は catalog が返した `{json_key: value}` を値のまま持ち、解釈しない。

mod add;
mod editor;
pub mod messages;
mod stage;
mod ui;

#[cfg(test)]
mod test_catalog;

pub use add::{AddKeyAction, EffectAddPane, EffectAddState};
pub use editor::{ChainKeyAction, EffectChainEditor};
pub use stage::{chain_json, stage_is_bypassed, stage_label, stage_with_bypass};
pub use ui::{draw, draw_chain_pane, EffectChainView};

/// `PageDown`/`PageUp` の 1 回あたりの移動段数。chain 一覧・追加 overlay の両方で使う。
pub const PAGE_STEP: isize = 10;

pub fn clamped_index(index: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    index.saturating_add_signed(delta).min(len - 1)
}
