//! サンプル MID モードの描画。MML 欄・matrix・左 pane は中身を灰色の `-` にし、
//! 右 pane に MID の全イベントを出す。一覧 overlay（`o`）は history overlay と同じ形。

use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY},
    ui::centered_rect_with_size,
};

use super::event_list::{visible_lines, EventRow};
use super::{focused_pane_block, pane_block, GuitarArticulationLayout, MATRIX_TITLE, MML_TITLE};
use crate::GuitarArticulationScreen;

const LIST_TITLE: &str = " Sample MIDI  j/k:選ぶ  Enter:読み込み  Esc:閉じる ";
/// MID モードで使えない欄の中身。
const UNUSED_TEXT: &str = "-";

/// MID モードの 4 つの pane。MID を開いていなければ何もしない。
pub(super) fn draw_panes(
    f: &mut Frame<'_>,
    layout: &GuitarArticulationLayout,
    screen: &GuitarArticulationScreen,
) {
    let Some(midi) = screen.sample_midi() else {
        return;
    };
    for (area, title) in [
        (layout.input, MML_TITLE),
        (layout.matrix, MATRIX_TITLE),
        (layout.plain, super::PLAIN_TITLE),
    ] {
        f.render_widget(
            Paragraph::new(Line::styled(UNUSED_TEXT, base_style().fg(MONOKAI_GRAY)))
                .block(pane_block(title)),
            area,
        );
    }
    let block = focused_pane_block(format!(" MID: {} (space) ", midi.name()));
    let height = block.inner(layout.converted).height as usize;
    let rows: Vec<EventRow> = midi
        .events()
        .iter()
        .map(|event| EventRow::new(event, true))
        .collect();
    let target = midi.group_event_index(screen.sample_midi_cursor());
    f.render_widget(
        Paragraph::new(visible_lines(&rows, height, target)).block(block),
        layout.converted,
    );
}

/// 一覧 overlay。file 名を 1 行 1 件で並べ、選んでいる行を反転する。閉じていれば何もしない。
pub(super) fn draw_list_overlay(f: &mut Frame<'_>, screen: &GuitarArticulationScreen) {
    let Some((files, selected)) = screen.sample_midi_list() else {
        return;
    };
    let area = overlay_rect(f.area(), files.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(LIST_TITLE)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_CYAN));
    let items: Vec<ListItem> = files
        .iter()
        .map(|path| {
            let name = path.file_name().unwrap_or(path.as_os_str());
            ListItem::new(name.to_string_lossy().into_owned())
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(block)
        .style(base_style().fg(MONOKAI_FG))
        .highlight_style(cursor_highlight_style(base_style().fg(MONOKAI_FG)));
    f.render_widget(Clear, area);
    f.render_stateful_widget(list, area, &mut state);
}

fn overlay_rect(area: Rect, row_count: usize) -> Rect {
    let width = area.width.saturating_sub(4).min(80);
    let height = u16::try_from(row_count.max(1) + 2)
        .unwrap_or(u16::MAX)
        .min(area.height.saturating_sub(2))
        .max(3);
    centered_rect_with_size(width, height, area)
}
