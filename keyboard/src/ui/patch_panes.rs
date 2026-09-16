//! Role / Preset / 音色 の 3 pane。MML overlay の patch selector と同じ見た目。

use ratatui::{
    layout::{Alignment, Constraint, Rect},
    text::Line,
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};

use cmrt_mml_overlay::ui::load_time_label;
use cmrt_tui_core::status::{base_style, visible_list_page_size, LIST_HIGHLIGHT_SYMBOL};
use cmrt_tui_core::theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_YELLOW};

use crate::{KeyboardPatchCatalog, KeyboardPatchCatalogStatus, PatchPaneFocus};

const CATEGORY_COLUMN_WIDTH: u16 = 12;
const LOAD_COLUMN_WIDTH: u16 = 7;

/// keyboard pane の右に残った幅を、overlay と同じ式で 3 つに割る。
pub(super) fn pane_widths(remaining: u16) -> [Constraint; 3] {
    [
        Constraint::Length((remaining / 5).clamp(12, 22)),
        Constraint::Length((remaining / 4).clamp(14, 30)),
        Constraint::Min(12),
    ]
}

pub(super) fn draw_patch_panes(
    catalog: &mut KeyboardPatchCatalog,
    f: &mut Frame<'_>,
    role_area: Rect,
    preset_area: Rect,
    patch_area: Rect,
) {
    catalog.sync_list_states(
        visible_list_page_size(role_area),
        visible_list_page_size(preset_area),
        // 枠内の 1 行は header が使う。
        visible_list_page_size(patch_area).saturating_sub(1).max(1),
    );
    draw_roles(catalog, f, role_area);
    draw_presets(catalog, f, preset_area);
    draw_patches(catalog, f, patch_area);
}

fn pane_block(title: String, focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(base_style())
        .border_style(base_style().fg(if focused {
            MONOKAI_YELLOW
        } else {
            MONOKAI_CYAN
        }))
}

fn draw_roles(catalog: &mut KeyboardPatchCatalog, f: &mut Frame<'_>, area: Rect) {
    let items = catalog
        .roles()
        .iter()
        .map(|role| ListItem::new(role.label()))
        .collect::<Vec<_>>();
    let title = format!(" Role ({}/{}) ", catalog.role_cursor() + 1, items.len());
    let focused = catalog.focus() == PatchPaneFocus::Role;
    f.render_stateful_widget(
        List::new(items)
            .style(base_style())
            .highlight_style(cursor_highlight_style(base_style()))
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL)
            .block(pane_block(title, focused)),
        area,
        catalog.role_list_state_mut(),
    );
}

fn draw_presets(catalog: &mut KeyboardPatchCatalog, f: &mut Frame<'_>, area: Rect) {
    let items = catalog
        .presets()
        .iter()
        .map(|preset| {
            let prefix = if preset.is_user { "+ " } else { "" };
            ListItem::new(format!("{prefix}{}", preset.label))
        })
        .collect::<Vec<_>>();
    let title = format!(" Preset ({}/{}) ", catalog.preset_cursor() + 1, items.len());
    let focused = catalog.focus() == PatchPaneFocus::Preset;
    f.render_stateful_widget(
        List::new(items)
            .style(base_style())
            .highlight_style(cursor_highlight_style(base_style()))
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL)
            .block(pane_block(title, focused)),
        area,
        catalog.preset_list_state_mut(),
    );
}

fn draw_patches(catalog: &mut KeyboardPatchCatalog, f: &mut Frame<'_>, area: Rect) {
    let rows = catalog
        .patches()
        .map(|patch| {
            let load = load_time_label(catalog.load_measurement(patch.display()));
            Row::new([
                Cell::from(patch.selector_category().unwrap_or("").to_string()),
                Cell::from(patch.display().to_string()),
                Cell::from(Line::from(load).alignment(Alignment::Right)),
            ])
        })
        .collect::<Vec<_>>();
    let title = format!(
        " Patches ({}/{}) ",
        catalog.selected_patch_index().map_or(0, |index| index + 1),
        rows.len()
    );
    let block = pane_block(title, catalog.focus() == PatchPaneFocus::Patches);
    if rows.is_empty() {
        // 一覧が無い理由は音色 pane の 1 行目に出す。Role / Preset は空のまま。
        let message = catalog_message(catalog.status());
        f.render_widget(
            Paragraph::new(message).style(base_style()).block(block),
            area,
        );
        return;
    }
    f.render_stateful_widget(
        Table::new(
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
        .style(base_style())
        .row_highlight_style(cursor_highlight_style(base_style()))
        .highlight_symbol(LIST_HIGHLIGHT_SYMBOL)
        .block(block),
        area,
        catalog.patch_table_state_mut(),
    );
}

fn catalog_message(status: &KeyboardPatchCatalogStatus) -> String {
    match status {
        KeyboardPatchCatalogStatus::Loading => "パッチを読み込み中...".to_string(),
        KeyboardPatchCatalogStatus::NotConfigured => {
            "patches_dirs が設定されていません".to_string()
        }
        KeyboardPatchCatalogStatus::Ready => "パッチが見つかりません".to_string(),
        KeyboardPatchCatalogStatus::Error(error) => format!("読み込み失敗: {error}"),
    }
}
