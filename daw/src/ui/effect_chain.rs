use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::{
    super::{overlays::effect_stage_label, DawApp, DawMode},
    MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY,
};
use crate::messages::effect_chain as message;
use cmrt_tui_core::theme::cursor_highlight_style;

pub(super) fn draw_effect_chain(f: &mut Frame, app: &DawApp, area: Rect) {
    let popup = cmrt_tui_core::ui::centered_rect(70, 70, area);
    f.render_widget(Clear, popup);
    let (title, footer) = if app.mode == DawMode::EffectChainAdd {
        (message::ADD_OVERLAY_TITLE, message::ADD_FOOTER)
    } else {
        (message::OVERLAY_TITLE, message::FOOTER)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(MONOKAI_CYAN))
        .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let state = &app.overlays.effect_chain;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} ", crate::tracks::track_label(state.track)),
                Style::default().fg(MONOKAI_GRAY),
            ),
            Span::raw(message::INSTRUMENT_LABEL),
            Span::raw(state.instrument.as_str()),
        ])),
        chunks[0],
    );

    let catalog = app.effect_plugins.catalog();
    let (items, selected): (Vec<ListItem<'static>>, Option<usize>) = if app.mode
        == DawMode::EffectChainAdd
    {
        let presets = catalog.map(|catalog| catalog.presets()).unwrap_or_default();
        (
            presets
                .iter()
                .enumerate()
                .map(|(index, preset)| list_item(preset.display.clone(), index == state.add_cursor))
                .collect(),
            (!presets.is_empty()).then_some(state.add_cursor),
        )
    } else if state.chain.is_empty() {
        let notice = if app.effect_preset_count() == 0 {
            message::NO_PRESETS
        } else {
            message::EMPTY_CHAIN
        };
        (
            vec![ListItem::new(notice).style(Style::default().fg(MONOKAI_GRAY))],
            None,
        )
    } else {
        (
            state
                .chain
                .iter()
                .enumerate()
                .map(|(index, stage)| {
                    list_item(
                        format!("{}. {}", index + 1, effect_stage_label(stage, catalog)),
                        index == state.cursor,
                    )
                })
                .collect(),
            Some(state.cursor),
        )
    };
    let mut list_state = ListState::default();
    list_state.select(selected);
    f.render_stateful_widget(List::new(items), chunks[1], &mut list_state);

    f.render_widget(
        Paragraph::new(footer).style(Style::default().fg(MONOKAI_CYAN)),
        chunks[2],
    );
}

fn list_item(text: String, is_selected: bool) -> ListItem<'static> {
    let prefix = if is_selected { "▶ " } else { "  " };
    let style = if is_selected {
        cursor_highlight_style(Style::default().fg(MONOKAI_FG))
    } else {
        Style::default().fg(MONOKAI_FG)
    };
    ListItem::new(format!("{prefix}{text}")).style(style)
}
