use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_GREEN},
};

use crate::{GridRow, GridSequencerScreen, GRID_STEPS};

/// patch name 欄の桁数。
const PATCH_WIDTH: usize = 24;
/// note on のセル記号と、休符のセル記号（1セルは記号+空白の2桁）。
const NOTE_CELL: &str = "# ";
const REST_CELL: &str = ". ";

pub(super) fn draw(screen: &GridSequencerScreen, f: &mut Frame<'_>, area: Rect) {
    let playhead = screen.state.step_index();
    let mut lines = vec![header_line()];
    lines.extend(
        screen
            .state
            .rows()
            .iter()
            .enumerate()
            .map(|(index, row)| row_line(index, row, playhead)),
    );
    f.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Grid Sequencer ")
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

fn header_line() -> Line<'static> {
    let style = base_style().fg(MONOKAI_GRAY);
    Line::from(vec![
        Span::styled(label_columns("#", "PATCH", "NOTE", "DUR"), style),
        Span::styled(step_ruler(), style),
    ])
}

fn row_line(index: usize, row: &GridRow, playhead: usize) -> Line<'static> {
    let mut spans = vec![Span::styled(
        label_columns(
            &(index + 1).to_string(),
            &truncate_patch(row.patch.as_deref(), PATCH_WIDTH),
            &row.note.to_string(),
            row.duration.label(),
        ),
        base_style(),
    )];
    for (step, on) in row.cells.iter().enumerate() {
        let color = if *on { MONOKAI_GREEN } else { MONOKAI_GRAY };
        let style = if step == playhead {
            cursor_highlight_style(base_style().fg(color))
        } else {
            base_style().fg(color)
        };
        spans.push(Span::styled(if *on { NOTE_CELL } else { REST_CELL }, style));
    }
    Line::from(spans)
}

/// grid の左に置く情報欄（行番号 / patch name / note number / 音長）。
fn label_columns(index: &str, patch: &str, note: &str, duration: &str) -> String {
    format!(
        " {index:>2} {patch:<width$} {note:>4} {duration:>5} ",
        width = PATCH_WIDTH
    )
}

/// 4ステップごとに1始まりの番号を出す目盛り。
fn step_ruler() -> String {
    (0..GRID_STEPS)
        .map(|step| {
            if step % 4 == 0 {
                format!("{:<2}", step + 1)
            } else {
                "  ".to_string()
            }
        })
        .collect()
}

/// 長い patch name は先頭を省略する（末尾のファイル名のほうが判別に役立つため）。
fn truncate_patch(patch: Option<&str>, width: usize) -> String {
    let Some(patch) = patch else {
        return "-".to_string();
    };
    let chars = patch.chars().collect::<Vec<_>>();
    if chars.len() <= width {
        return patch.to_string();
    }
    let tail = chars[chars.len() + 1 - width..].iter().collect::<String>();
    format!("…{tail}")
}
