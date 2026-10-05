//! note 昇順の 16 step matrix と、カーソル追従の縦スクロール。

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_GREEN},
    ui::draw_frame_background,
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{DrumSequencerScreen, DRUM_STEPS};

const NOTE_WIDTH: usize = 4;
const STEP_WIDTH: usize = 3;

#[derive(Clone, Copy)]
struct DrumSequencerLayout {
    kit: Rect,
    steps: Rect,
    matrix: Rect,
    cursor: Rect,
    status: Rect,
    keys: Rect,
}

fn screen_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(" Drum Sequencer ")
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_CYAN))
}

fn layout_for(area: Rect) -> DrumSequencerLayout {
    let inner = screen_block().inner(area);
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);
    DrumSequencerLayout {
        kit: rows[0],
        steps: rows[1],
        matrix: rows[2],
        cursor: rows[3],
        status: rows[4],
        keys: rows[5],
    }
}

/// viewport だけを更新する。kit や入力を初期化せず、画面往復中も同じ状態を使う。
pub fn draw(screen: &mut DrumSequencerScreen, frame: &mut Frame<'_>) {
    draw_with_status(screen, "", frame);
}

/// host の sender 状態を表示する。音色 load や失敗の情報は host が所有する。
pub fn draw_with_status(screen: &mut DrumSequencerScreen, status: &str, frame: &mut Frame<'_>) {
    draw_frame_background(frame);
    let area = frame.area();
    let layout = layout_for(area);
    frame.render_widget(screen_block(), area);
    let kit = screen.kit_name().unwrap_or("未選択 — t:kit 選択");
    frame.render_widget(
        Paragraph::new(format!("Kit: {kit}")).style(base_style()),
        layout.kit,
    );
    if screen.notes().is_empty() {
        draw_empty(screen, frame, layout.matrix);
    } else {
        draw_steps(frame, layout.steps);
        draw_matrix(screen, frame, layout.matrix);
    }
    let cursor = screen.cursor_note().map_or_else(String::new, |note| {
        format!(
            "Note {note}  Step {}/{}  {}",
            screen.cursor_step() + 1,
            DRUM_STEPS,
            if screen.cell_on(note, screen.cursor_step()) {
                "ON"
            } else {
                "OFF"
            }
        )
    });
    frame.render_widget(Paragraph::new(cursor).style(base_style()), layout.cursor);
    frame.render_widget(Paragraph::new(status).style(base_style()), layout.status);
    frame.render_widget(
        Paragraph::new("hjkl:移動 Space/Enter:切替 t:kit Shift+P:preview Ctrl+G:画面 q:終了")
            .style(base_style().fg(MONOKAI_GRAY)),
        layout.keys,
    );
}

fn draw_empty(screen: &DrumSequencerScreen, frame: &mut Frame<'_>, area: Rect) {
    let message = if screen.kit_name().is_none() {
        "t で Drum kit を選択してください。"
    } else if !screen.notes_known() {
        "この kit の note 一覧がありません。cmrt build-patch-catalog-cache で catalog を再生成してください。"
    } else {
        "この kit には note-on の割当がありません。t で別の kit を選択してください。"
    };
    frame.render_widget(
        Paragraph::new(message)
            .style(base_style().fg(MONOKAI_GRAY))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_steps(frame: &mut Frame<'_>, area: Rect) {
    let mut spans = vec![Span::raw(format!("{:NOTE_WIDTH$}", "Note"))];
    spans.extend((1..=DRUM_STEPS).map(|step| Span::raw(format!("{step:^STEP_WIDTH$}"))));
    frame.render_widget(Paragraph::new(Line::from(spans)).style(base_style()), area);
}

fn draw_matrix(screen: &mut DrumSequencerScreen, frame: &mut Frame<'_>, area: Rect) {
    let rows = screen.visible_rows(usize::from(area.height));
    let lines: Vec<Line<'static>> = rows
        .map(|row| {
            let note = screen.notes()[row];
            let mut spans = vec![Span::raw(format!("{note:>3} "))];
            for step in 0..DRUM_STEPS {
                let on = screen.cell_on(note, step);
                let selected = row == screen.cursor_row && step == screen.cursor_step();
                let mark = if on { 'x' } else { '.' };
                let mut style = base_style().fg(if on { MONOKAI_GREEN } else { MONOKAI_GRAY });
                let text = if selected {
                    style = cursor_highlight_style(style);
                    format!("[{mark}]")
                } else {
                    format!(" {mark} ")
                };
                spans.push(Span::styled(text, style));
            }
            Line::from(spans)
        })
        .collect();
    // Paragraph clips narrow terminals; the matrix always retains all 16 steps.
    frame.render_widget(Paragraph::new(lines).style(base_style()), area);
}

#[cfg(test)]
mod tests;
