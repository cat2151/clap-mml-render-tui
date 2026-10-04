//! Role / Preset / 音色 の 3 pane。MML overlay の patch selector と同じ見た目。

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Color,
    text::Line,
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};

use cmrt_patch_select::ui::load_time_label;
use cmrt_tui_core::status::{base_style, visible_list_page_size, LIST_HIGHLIGHT_SYMBOL};
use cmrt_tui_core::text_input::{
    build_query_textarea_widget, single_line_textarea_cursor_position,
};
use cmrt_tui_core::theme::{cursor_highlight_style, MONOKAI_FG, MONOKAI_YELLOW};

use crate::{
    KeyboardPatchCatalog, KeyboardPatchCatalogStatus, KeyboardPatchFilterInput, PatchPaneFocus,
};

/// 絞り込み入力欄の高さ（枠 + 中身 1 行）。
const FILTER_INPUT_HEIGHT: u16 = 3;

const CATEGORY_COLUMN_WIDTH: u16 = 12;
const LOAD_COLUMN_WIDTH: u16 = 7;
/// Patches 表の音色名以外の幅。枠 2 + `▶ ` 2 + Category + Load + 列の間 2。
const PATCH_TABLE_FIXED_WIDTH: u16 = 2 + 2 + CATEGORY_COLUMN_WIDTH + LOAD_COLUMN_WIDTH + 2;

/// keyboard pane の右に残った幅のうち Effect pane に必ず残す幅。残り幅の 1/5 を 16〜28 に収める。
fn effect_pane_min_width(remaining: u16) -> u16 {
    (remaining / 5).clamp(16, 28)
}

/// Patch selector の中の Role / Preset / Patches の幅。
pub(super) struct PatchSelectorWidths {
    role: u16,
    preset: u16,
    patches: u16,
}

impl PatchSelectorWidths {
    /// keyboard pane の右に残った幅 `remaining` から決める。Role / Preset は overlay と同じ式で、
    /// Patches は最後に測った一覧の最長名に合わせる（いつ測るかは `catalog/list_width.rs`）。
    /// Patches が使わない幅は Effect pane に回る。一覧が空の間は、理由の文言が切れないよう最大まで取る。
    pub(super) fn new(catalog: &KeyboardPatchCatalog, remaining: u16) -> Self {
        let inner_max = remaining
            .saturating_sub(effect_pane_min_width(remaining))
            .saturating_sub(2);
        let role = (inner_max / 5).clamp(12, 22);
        let preset = (inner_max / 4).clamp(14, 30);
        let patches_max = inner_max.saturating_sub(role + preset).max(12);
        let patches = match catalog.patch_name_width() {
            0 => patches_max,
            name => u16::try_from(name)
                .unwrap_or(u16::MAX)
                .saturating_add(PATCH_TABLE_FIXED_WIDTH)
                .min(patches_max),
        };
        Self {
            role,
            preset,
            patches,
        }
    }

    /// 外枠を含めた Patch selector 全体の幅。
    pub(super) fn total(&self) -> u16 {
        self.role + self.preset + self.patches + 2
    }
}

/// Role / Preset / Patches を 1 つの「Patch selector」の枠で囲んで描く。
/// focus は内側の各 pane の枠の色が示すので、外枠は常に `MONOKAI_FG`。
pub(super) fn draw_patch_selector(
    catalog: &mut KeyboardPatchCatalog,
    filter_input: &KeyboardPatchFilterInput<'_>,
    widths: &PatchSelectorWidths,
    f: &mut Frame<'_>,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Patch selector ")
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_FG));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(widths.role),
            Constraint::Length(widths.preset),
            Constraint::Min(widths.patches),
        ])
        .split(inner);
    draw_patch_panes(catalog, filter_input, f, panes[0], panes[1], panes[2]);
}

fn draw_patch_panes(
    catalog: &mut KeyboardPatchCatalog,
    filter_input: &KeyboardPatchFilterInput<'_>,
    f: &mut Frame<'_>,
    role_area: Rect,
    preset_area: Rect,
    patch_area: Rect,
) {
    // 枠内の 1 行は header が使う。入力欄を重ねている間はその分も隠れる。
    let hidden_rows = if filter_input.is_active() {
        1 + FILTER_INPUT_HEIGHT as usize
    } else {
        1
    };
    catalog.sync_list_states(
        visible_list_page_size(role_area),
        visible_list_page_size(preset_area),
        visible_list_page_size(patch_area)
            .saturating_sub(hidden_rows)
            .max(1),
    );
    draw_roles(catalog, f, role_area);
    draw_presets(catalog, f, preset_area);
    draw_patches(catalog, f, patch_area);
    draw_filter_input(filter_input, f, patch_area);
}

/// 入力中の絞り込み欄を、Patches pane の枠の内側の最下段へ重ねる。
fn draw_filter_input(input: &KeyboardPatchFilterInput<'_>, f: &mut Frame<'_>, patch_area: Rect) {
    if !input.is_active() {
        return;
    }
    let inner = Block::default().borders(Borders::ALL).inner(patch_area);
    let height = FILTER_INPUT_HEIGHT.min(inner.height);
    let area = Rect::new(
        inner.x,
        inner.y + inner.height - height,
        inner.width,
        height,
    );
    let border_color = if input.is_invalid() {
        Color::Red
    } else {
        MONOKAI_YELLOW
    };
    let value = input.value();
    f.render_widget(
        &build_query_textarea_widget(
            input.textarea(),
            &value,
            " /filter  Enter:確定  Esc:戻す ",
            "Regex (空白=AND  -語:除外)  plugin:名前",
            border_color,
        ),
        area,
    );
    f.set_cursor_position(single_line_textarea_cursor_position(area, input.textarea()));
}

pub(super) fn pane_block(title: String, focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(base_style())
        .border_style(base_style().fg(if focused { MONOKAI_YELLOW } else { MONOKAI_FG }))
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
    let filter = match catalog.filter() {
        "" => String::new(),
        condition => format!(" /{condition}"),
    };
    let title = format!(
        " Patches ({}/{}){filter} ",
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
