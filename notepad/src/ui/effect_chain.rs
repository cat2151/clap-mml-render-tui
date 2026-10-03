//! EFFECT CHAIN overlay（`x`）の描画。overlay 本体は `cmrt_effect_chain_select::draw`。

use ratatui::{text::Line, Frame};

use crate::{Mode, NotepadScreen};

const INSTRUMENT_LABEL: &str = "instrument: ";

/// `mode` は overlay を開いている側のモード（help を重ねているなら help を開いた側）。
pub(super) fn draw_effect_chain(app: &NotepadScreen<'_>, f: &mut Frame, mode: Mode) {
    let instrument = app.current_line_patch_name().unwrap_or_default();
    cmrt_effect_chain_select::draw(
        f,
        f.area(),
        &app.effect_chain,
        app.effect_plugins.catalog(),
        cmrt_effect_chain_select::EffectChainView {
            adding: mode == Mode::EffectChainAdd,
            header: Line::from(format!("{INSTRUMENT_LABEL}{instrument}")),
            error: None,
        },
    );
}
