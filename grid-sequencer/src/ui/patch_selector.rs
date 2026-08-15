use ratatui::{
    style::Style,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG},
};

use crate::{patch_selector::PatchSelectorLayout, GridSequencerScreen};

pub(super) fn draw(f: &mut Frame<'_>, screen: &GridSequencerScreen) {
    let Some(selector) = screen.patch_selector.as_ref() else {
        return;
    };
    let layout = PatchSelectorLayout::new(f.area(), selector.name_search_visible());
    f.render_widget(Clear, layout.popup);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" instance {} patch select ", selector.instance + 1))
            .style(base_style().fg(MONOKAI_FG).bg(MONOKAI_BG))
            .border_style(base_style().fg(MONOKAI_CYAN)),
        layout.popup,
    );

    if let Some(filter_area) = layout.name_search {
        let filter = cmrt_tui_core::text_input::build_query_textarea_widget(
            selector.name_query_textarea(),
            selector.name_query(),
            " Patch name filter ",
            "type patch name",
            MONOKAI_CYAN,
        );
        f.render_widget(&filter, filter_area);
        if selector.name_search_active {
            f.set_cursor_position(
                cmrt_tui_core::text_input::single_line_textarea_cursor_position(
                    filter_area,
                    selector.name_query_textarea(),
                ),
            );
        }
    }

    let categories = selector
        .category_range(&layout)
        .map(|index| {
            let category = &selector.categories[index];
            let label = if selector.has_name_query() {
                format!("{} ({})", category.name, category.patches.len())
            } else {
                category.name.clone()
            };
            item(&label, index == selector.category_cursor)
        })
        .collect::<Vec<_>>();
    f.render_widget(
        List::new(categories).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Categories ({}) ", selector.categories.len()))
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        layout.category_pane,
    );

    let category = selector.selected_category();
    let patches = selector
        .patch_range(&layout)
        .map(|index| item(&category.patches[index], index == selector.patch_cursor))
        .collect::<Vec<_>>();
    f.render_widget(
        List::new(patches).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ({}) ", category.name, category.patches.len()))
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        layout.patch_pane,
    );
    let hint = if selector.name_search_active {
        " type:filter  Enter:confirm  Esc:cancel input"
    } else {
        " wheel/↑↓:preview  ←→:category  r:random  /:filter  click/Enter:apply  Esc/right:cancel"
    };
    f.render_widget(
        Paragraph::new(hint).style(base_style().fg(MONOKAI_CYAN)),
        layout.hint,
    );
}

fn item(text: &str, selected: bool) -> ListItem<'static> {
    let prefix = if selected { "▶ " } else { "  " };
    let style = if selected {
        cursor_highlight_style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG))
    } else {
        Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG)
    };
    ListItem::new(format!("{prefix}{text}")).style(style)
}
