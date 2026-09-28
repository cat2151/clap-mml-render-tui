//! auto reverb の表示行と、ルール overlay・effect list の描画。

use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
    Frame,
};
use serde_json::Value;

use cmrt_tui_core::{
    status::{base_style, LIST_HIGHLIGHT_SYMBOL},
    theme::{cursor_highlight_style, MONOKAI_FG, MONOKAI_YELLOW},
    ui::centered_rect,
};

use crate::auto_reverb::AutoReverb;
use crate::patch_select::{AutoReverbPanel, AutoReverbStatus, EffectList, PatchSelect};

const RULE_NAME_COLUMN_WIDTH: u16 = 24;

/// 表示行の文言。host が auto reverb を扱わなければ `None`（行を取らない）。
pub(super) fn status_line(select: &PatchSelect<'_>) -> Option<String> {
    Some(status_text(&select.auto_reverb_status()?))
}

/// `status` の表示行の文言。
pub(super) fn status_text(status: &AutoReverbStatus) -> String {
    match status {
        AutoReverbStatus::ManualReverb { reverbs } if reverbs.is_empty() => {
            "manual reverb: dry".to_string()
        }
        AutoReverbStatus::ManualReverb { reverbs } => {
            format!("manual reverb: {}", reverbs.join(" + "))
        }
        AutoReverbStatus::Resolved(AutoReverb::Apply {
            effect_name, row, ..
        }) => format!("auto reverb: {effect_name} ({row})"),
        AutoReverbStatus::Resolved(AutoReverb::Dry { row }) => format!("auto reverb: dry ({row})"),
        AutoReverbStatus::Resolved(AutoReverb::Builtin) => String::new(),
        AutoReverbStatus::Resolved(AutoReverb::Off) => "auto reverb: off(dry)".to_string(),
        AutoReverbStatus::Resolved(AutoReverb::NoCatalog) => {
            "auto reverb: -（effect catalog 無し）".to_string()
        }
        AutoReverbStatus::NoPatch => "auto reverb: -".to_string(),
    }
}

pub(super) fn draw_status(select: &PatchSelect<'_>, frame: &mut Frame<'_>, area: Rect) {
    if let Some(line) = status_line(select) {
        frame.render_widget(Paragraph::new(line).style(base_style()), area);
    }
}

/// ルールの effect の表示名。`{json_key: value}` の value を出す。
fn effect_label(effect: Option<&Value>) -> String {
    let Some(effect) = effect else {
        return "dry".to_string();
    };
    effect
        .as_object()
        .and_then(|object| object.values().next())
        .and_then(Value::as_str)
        .map_or_else(|| effect.to_string(), str::to_string)
}

/// ルール overlay が開いていれば `area` の中央へ描き、effect list が開いていればさらに手前へ描く。
pub(super) fn draw_rules_overlay(panel: &AutoReverbPanel, frame: &mut Frame<'_>, area: Rect) {
    let Some(overlay) = panel.overlay() else {
        return;
    };
    let rules = panel.rules();
    let overlay_area = centered_rect(70, 90, area);
    let rows = overlay
        .shown_rows()
        .iter()
        .map(|index| {
            let (row, effect) = &rules.rows()[*index];
            Row::new([
                Cell::from(row.name()),
                Cell::from(effect_label(effect.as_ref())),
            ])
        })
        .collect::<Vec<_>>();
    draw_list_table(
        frame,
        overlay_area,
        " auto reverb ルール  j/k:行  Enter:effect  Esc:保存して閉じる ",
        rows,
        &[
            Constraint::Length(RULE_NAME_COLUMN_WIDTH),
            Constraint::Fill(1),
        ],
        overlay.cursor(),
        overlay.effect_list().is_none(),
    );
    if let Some(list) = overlay.effect_list() {
        draw_effect_list(list, frame, centered_rect(60, 80, overlay_area));
    }
}

fn draw_effect_list(list: &EffectList, frame: &mut Frame<'_>, area: Rect) {
    let rows = list
        .choices()
        .iter()
        .map(|choice| Row::new([Cell::from(choice.label.as_str())]))
        .collect::<Vec<_>>();
    draw_list_table(
        frame,
        area,
        " effect  j/k PgUp/PgDn Home/End:選ぶ  Enter:決定  Esc:戻る ",
        rows,
        &[Constraint::Fill(1)],
        list.cursor(),
        true,
    );
}

fn draw_list_table(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    rows: Vec<Row<'_>>,
    widths: &[Constraint],
    cursor: usize,
    focused: bool,
) {
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title.to_string())
        .style(base_style())
        .border_style(base_style().fg(if focused { MONOKAI_YELLOW } else { MONOKAI_FG }));
    let mut state = TableState::default().with_selected(Some(cursor));
    frame.render_stateful_widget(
        Table::new(rows, widths.to_vec())
            .block(block)
            .row_highlight_style(cursor_highlight_style(Style::default().fg(MONOKAI_FG)))
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        area,
        &mut state,
    );
}

#[cfg(test)]
mod tests;
