use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    Frame,
};

use super::{
    super::{DawApp, DawMode},
    MONOKAI_GRAY,
};
use crate::messages::effect_chain as message;

pub(super) fn draw_effect_chain(f: &mut Frame, app: &DawApp, area: Rect) {
    // ヘルプを重ねている間も、ヘルプを開いた側の画面を下に描く。
    let state = &app.overlays.effect_chain;
    let adding = app.mode == DawMode::EffectChainAdd
        || (app.mode == DawMode::Help && app.help_origin == DawMode::EffectChainAdd);
    let header = Line::from(vec![
        Span::styled(
            format!("{} ", crate::tracks::track_label(state.track)),
            Style::default().fg(MONOKAI_GRAY),
        ),
        Span::raw(message::INSTRUMENT_LABEL),
        Span::raw(state.instrument.as_str()),
    ]);
    cmrt_effect_chain_select::draw(
        f,
        area,
        &state.editor,
        app.effect_plugins.catalog(),
        cmrt_effect_chain_select::EffectChainView {
            adding,
            header,
            error: state.preview_error.as_deref(),
        },
    );
}
