//! 奏法リスト overlay（`t`）。列ルールを matrix と同じ 5 段に分け、段の中の項目を横に並べる。
//! KS の段はグループごとに 3 行へ折り返す。各段の左端は「なし」（段のルールが全部 OFF）。
//! 項目は overlay の文字と名前で、グループの色で塗る（ON だが効かない項目は灰色）。
//! ON だが効かない項目があれば、下の枠にその項目と列の奏法を出す。
//! 選んでいる段の選ばれている項目を反転し、他の段の ON の項目は `[` `]` で囲む。
//! 1 行に収まらない行は、選ばれている項目が見えるように横へずらす。
//! 枠の内側の最下段は絞り込みの欄で、入力中か条件が空でないときに `/` と入力欄を出す。

use ratatui::{
    layout::{Position, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_PINK},
    ui::centered_rect_with_size,
};

use super::{
    matrix::{ineffective_notice, rule_cell},
    RuleLane, RuleRow, RULE_LANES,
};
use crate::{GuitarArticulationScreen, Rule};

const HINT: &str = "j/k:段 h/l:選んでON 文字:ON/OFF space:試聴 /:絞り込み Enter/Esc:閉じる";
const NONE_TEXT: &str = "なし";

/// overlay の 1 行。
struct ListLine {
    lane: RuleLane,
    rules: Vec<&'static RuleRow>,
    /// 段の先頭の行か。先頭なら左端に「なし」を置き、続きの行は同じ幅だけ空ける。
    leads: bool,
}

/// 段の項目を行に分ける。KS の段だけグループごとに折り返す。
fn lines_of(lane: RuleLane, rules: &[&'static RuleRow]) -> Vec<ListLine> {
    let chunks: Vec<&[&'static RuleRow]> = if lane == RuleLane::KeySwitch {
        rules.chunk_by(|a, b| a.group == b.group).collect()
    } else {
        vec![rules]
    };
    chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| ListLine {
            lane,
            rules: chunk.to_vec(),
            leads: index == 0,
        })
        .collect()
}

/// 絞り込みが無いときの行。overlay の大きさは、絞り込んでもこれで決めたまま変えない。
fn full_lines() -> Vec<ListLine> {
    RULE_LANES
        .iter()
        .flat_map(|&lane| lines_of(lane, &lane.rule_rows().collect::<Vec<_>>()))
        .collect()
}

/// 行の桁数。`[` `]` で囲んでも囲まなくても同じ。
fn line_width(line: &ListLine) -> usize {
    let none_width = Span::raw(format!(" {NONE_TEXT} ")).width();
    let items: usize = line
        .rules
        .iter()
        .map(|rule_row| Span::raw(format!(" {}:{} ", rule_row.overlay_key, rule_row.name)).width())
        .sum();
    none_width + items
}

pub(super) fn draw_overlay(f: &mut Frame<'_>, screen: &GuitarArticulationScreen) {
    if !screen.rule_list_open() {
        return;
    }
    let column = screen.cursor();
    let title = format!(" 奏法リスト 列 {}  {HINT} ", column + 1);
    let full_lines = full_lines();
    let widest = full_lines.iter().map(line_width).max().unwrap_or(0);
    let width = (Line::from(title.as_str()).width().max(widest) as u16 + 2).min(f.area().width);
    // 枠 2 + 絞り込みの欄 1。
    let height = (full_lines.len() as u16 + 3).min(f.area().height);
    let area: Rect = centered_rect_with_size(width, height, f.area());
    let mut block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_CYAN));
    if let Some(notice) = ineffective_notice(screen, column) {
        block = block.title_bottom(Line::styled(notice, base_style().fg(MONOKAI_PINK)));
    }
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);
    if inner.height == 0 {
        return;
    }
    let list_height = inner.height - 1;
    let selected_lane = screen.rule_list_lane();
    let mut y = inner.y;
    for listed in screen.rule_list_lanes() {
        let choice = screen.rule_list_choice(&listed);
        for line in lines_of(listed.lane, &listed.rules) {
            if y >= inner.y + list_height {
                break;
            }
            let selected = selected_lane == Some(line.lane);
            let (text, visible) = render_line(screen, column, &line, selected, choice);
            let offset = visible.map_or(0, |(start, end)| {
                end.saturating_sub(usize::from(inner.width)).min(start)
            });
            f.render_widget(
                Paragraph::new(text)
                    .style(base_style().fg(MONOKAI_FG))
                    .scroll((0, offset as u16)),
                Rect {
                    y,
                    height: 1,
                    ..inner
                },
            );
            y += 1;
        }
    }
    let show_filter =
        screen.rule_list_filter_input_active() || !screen.rule_list_query().is_empty();
    if show_filter {
        let filter_area = Rect {
            y: inner.y + list_height,
            height: 1,
            ..inner
        };
        draw_filter(f, filter_area, screen);
    }
}

/// 1 行を描く。選ばれている項目がこの行にあれば、その桁の範囲（始まり, 終わり）も返す。
fn render_line(
    screen: &GuitarArticulationScreen,
    column: usize,
    line: &ListLine,
    selected: bool,
    choice: Option<Rule>,
) -> (Line<'static>, Option<(usize, usize)>) {
    let mut spans = Vec::new();
    let mut x = 0;
    let mut visible = None;
    let mut push = |text: String, style: Style, chosen: bool| {
        let span = Span::styled(text, style);
        let span_width = span.width();
        if chosen {
            visible = Some((x, x + span_width));
        }
        x += span_width;
        spans.push(span);
    };
    let none_item = format!(" {NONE_TEXT} ");
    if line.leads {
        let chosen = selected && choice.is_none();
        push(
            none_item,
            item_style(base_style().fg(MONOKAI_FG), chosen),
            chosen,
        );
    } else {
        let width = Span::raw(none_item).width();
        push(" ".repeat(width), base_style(), false);
    }
    for rule_row in &line.rules {
        let on = screen.rules().is_on(column, rule_row.rule);
        let style = if on {
            rule_cell(screen, column, rule_row).1
        } else {
            base_style().fg(rule_row.group.color())
        };
        let chosen = selected && choice == Some(rule_row.rule);
        let (open, close) = if on && !selected {
            ('[', ']')
        } else {
            (' ', ' ')
        };
        push(
            format!("{open}{}:{}{close}", rule_row.overlay_key, rule_row.name),
            item_style(style, chosen),
            chosen,
        );
    }
    (Line::from(spans), visible)
}

fn item_style(style: Style, chosen: bool) -> Style {
    if chosen {
        cursor_highlight_style(style)
    } else {
        style
    }
}

/// 最下段の `/` と入力欄。入力中は textarea を、確定後は条件を灰色の文字で描く。
fn draw_filter(f: &mut Frame<'_>, area: Rect, screen: &GuitarArticulationScreen) {
    let prompt = Rect {
        width: 2.min(area.width),
        ..area
    };
    f.render_widget(Paragraph::new(" /").style(base_style()), prompt);
    let field = Rect {
        x: area.x + prompt.width,
        width: area.width - prompt.width,
        ..area
    };
    match screen.rule_list_filter_textarea() {
        Some(textarea) => {
            f.render_widget(textarea, field);
            let col = textarea.cursor().1 as u16;
            if field.width > 0 {
                f.set_cursor_position(Position::new(field.x + col.min(field.width - 1), field.y));
            }
        }
        None => f.render_widget(
            Paragraph::new(screen.rule_list_query().to_string())
                .style(base_style().fg(MONOKAI_GRAY)),
            field,
        ),
    }
}
