//! 右端の Effect pane と、`a` / `r` で開く追加・差し替え overlay。

use ratatui::{layout::Rect, text::Line, Frame};

use cmrt_effect_chain_select::{draw, draw_chain_pane, EffectChainView};

use super::patch_panes::pane_block;
use crate::{KeyboardScreen, PatchPaneFocus};

pub(super) fn draw_effect_pane(screen: &KeyboardScreen<'_>, f: &mut Frame<'_>, area: Rect) {
    let pane = screen.effect_pane();
    let focused = screen.state.patch_catalog.focus() == PatchPaneFocus::Effect;
    let block = pane_block(format!(" Effect ({}) ", pane.chain().len()), focused);
    // catalog の初回の参照は走査を待つので、段の名前が要るときだけ引く。
    let catalog = if pane.chain().is_empty() {
        None
    } else {
        screen.effect_plugins().catalog()
    };
    let notice = pane.empty_notice(screen.effect_plugins().is_available());
    draw_chain_pane(f, area, block, pane.editor(), catalog, notice);
}

pub(super) fn draw_effect_add_overlay(screen: &KeyboardScreen<'_>, f: &mut Frame<'_>, area: Rect) {
    let pane = screen.effect_pane();
    if !pane.is_adding() {
        return;
    }
    let header = Line::from(format!(
        "keyboard: {}",
        screen.state.patch().unwrap_or("init saw")
    ));
    draw(
        f,
        area,
        pane.editor(),
        screen.effect_plugins().catalog(),
        EffectChainView {
            adding: true,
            header,
            error: None,
        },
    );
}
