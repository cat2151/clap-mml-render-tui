use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, TableState},
    Frame,
};

use cmrt_mml_overlay::ui::load_time_label;
use cmrt_tui_core::{
    status::{base_style, LIST_HIGHLIGHT_SYMBOL},
    theme::{
        cursor_highlight_style, MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_PINK, MONOKAI_YELLOW,
    },
};

use crate::{
    patch_selector::{PatchPaneFocus, PatchSelector, PatchSelectorLayout},
    GridSequencerScreen,
};

const QUERY_PLACEHOLDER: &str = r"例: warm pad|strings";
const CATEGORY_COLUMN_WIDTH: u16 = 12;
const LOAD_COLUMN_WIDTH: u16 = 7;

pub(super) fn draw(f: &mut Frame<'_>, screen: &GridSequencerScreen) {
    let Some(selector) = screen.patch_selector.as_ref() else {
        return;
    };
    let layout = PatchSelectorLayout::new(f.area(), selector.filter_visible());
    f.render_widget(Clear, layout.popup);
    // 案内は枠の下辺へ載せる。中の layout を動かさないので、pane の行数も
    // クリック判定（`PatchSelectorLayout`）も 1 ドットも変わらない。
    let mut block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" instance {} patch select ", selector.instance + 1))
        .style(base_style().fg(MONOKAI_FG).bg(MONOKAI_BG))
        .border_style(base_style().fg(MONOKAI_CYAN));
    for note in selector.catalog_notes() {
        block = block
            .title_bottom(Line::from(format!(" {note} ")).style(base_style().fg(MONOKAI_PINK)));
    }
    f.render_widget(block, layout.popup);

    if let Some(query_area) = layout.query {
        draw_query(f, selector, query_area);
    }

    let roles = selector
        .role_range(&layout)
        .map(|index| {
            item(
                selector.roles()[index].label(),
                index == selector.role_cursor(),
            )
        })
        .collect::<Vec<_>>();
    f.render_widget(
        pane_list(roles, " Role ", selector.focus() == PatchPaneFocus::Role),
        layout.role_pane,
    );

    let presets = selector
        .preset_range(&layout)
        .map(|index| {
            let preset = &selector.presets()[index];
            let label = if preset.is_user {
                format!("+ {}", preset.label)
            } else {
                preset.label.clone()
            };
            item(&label, index == selector.preset_cursor())
        })
        .collect::<Vec<_>>();
    f.render_widget(
        pane_list(
            presets,
            " Preset ",
            selector.focus() == PatchPaneFocus::Preset,
        ),
        layout.preset_pane,
    );

    draw_patches(f, selector, &layout);

    let hint = if selector.filter_editing() {
        " type:regex(空白=AND)  Enter:confirm  Esc:revert"
    } else {
        " h/l:pane  j/k:move  r:random  /:regex  click/Enter:apply  Esc/q/right:cancel"
    };
    f.render_widget(
        Paragraph::new(hint).style(base_style().fg(MONOKAI_CYAN)),
        layout.hint,
    );
}

fn draw_query(f: &mut Frame<'_>, selector: &PatchSelector, area: Rect) {
    let value = cmrt_tui_core::text_input::textarea_value(selector.query_textarea());
    let query = cmrt_tui_core::text_input::build_query_textarea_widget(
        selector.query_textarea(),
        &value,
        " Regex ",
        QUERY_PLACEHOLDER,
        MONOKAI_CYAN,
    );
    f.render_widget(&query, area);
    if selector.filter_editing() {
        f.set_cursor_position(
            cmrt_tui_core::text_input::single_line_textarea_cursor_position(
                area,
                selector.query_textarea(),
            ),
        );
    }
}

/// 音色 pane。`Category | Patch | Load` の表で、表示範囲は `patch_range` が決める。
fn draw_patches(f: &mut Frame<'_>, selector: &PatchSelector, layout: &PatchSelectorLayout) {
    let range = selector.patch_range(layout);
    let selected = range
        .contains(&selector.patch_cursor)
        .then(|| selector.patch_cursor - range.start);
    let rows = range
        .filter_map(|index| selector.filtered_entry(index))
        .map(|entry| {
            Row::new([
                Cell::from(entry.selector_category().unwrap_or("")),
                Cell::from(entry.display()),
                Cell::from(
                    Line::from(load_time_label(selector.load_measurement(entry.display())))
                        .alignment(Alignment::Right),
                ),
            ])
            .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG))
        })
        .collect::<Vec<_>>();
    let table = Table::new(
        rows,
        [
            Constraint::Length(CATEGORY_COLUMN_WIDTH),
            Constraint::Fill(1),
            Constraint::Length(LOAD_COLUMN_WIDTH),
        ],
    )
    .header(Row::new([
        Cell::from("Category"),
        Cell::from("Patch"),
        Cell::from(Line::from("Load").alignment(Alignment::Right)),
    ]))
    .block(pane_block(
        patch_title(selector),
        selector.focus() == PatchPaneFocus::Patches,
    ))
    .row_highlight_style(cursor_highlight_style(
        Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG),
    ))
    .highlight_symbol(LIST_HIGHLIGHT_SYMBOL);
    let mut state = TableState::default().with_selected(selected);
    f.render_stateful_widget(table, layout.patch_pane, &mut state);
}

fn patch_title(selector: &PatchSelector) -> String {
    if selector.filter_error().is_some() {
        return " Regex error ".to_string();
    }
    let list_len = selector.filtered_len();
    let position = if list_len == 0 {
        0
    } else {
        selector.patch_cursor + 1
    };
    format!(" Patches ({position}/{list_len}/{}) ", selector.total())
}

fn pane_list(
    items: Vec<ListItem<'static>>,
    title: impl Into<String>,
    focused: bool,
) -> List<'static> {
    List::new(items).block(pane_block(title, focused))
}

/// focus 中の pane だけ枠を黄色にする。
fn pane_block(title: impl Into<String>, focused: bool) -> Block<'static> {
    let border = if focused {
        MONOKAI_YELLOW
    } else {
        MONOKAI_CYAN
    };
    Block::default()
        .borders(Borders::ALL)
        .title(title.into())
        .border_style(base_style().fg(border))
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
