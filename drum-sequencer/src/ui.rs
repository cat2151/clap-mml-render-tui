//! 高い note を上に描く 16 step matrix と、カーソル追従の縦スクロール。
//! 左列は note number と構成音名。名前は列幅で切り、step 列は端末幅で変えない。
//! 音長は打点の後ろへ `-` で描き、同じ note の次の打点か小節の終わりで止める。
//! 拍の区切りが見えるよう、偶数拍（4 step ごと交互）の背景を塗る。

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_YELLOW},
    ui::draw_frame_background,
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{DrumSequencerScreen, KitResolution, DRUM_STEPS};

mod help;

/// 左列の最小幅。note number と区切りの空白だけが入る。
const NOTE_WIDTH: u16 = 4;
/// 左列の最大幅。広い端末でも step 列を左列から離しすぎない。
const LABEL_MAX_WIDTH: u16 = 28;
const STEP_WIDTH: usize = 3;
const STEPS_PER_BEAT: usize = 4;
/// 偶数拍の背景。カーソルの背景より暗くし、カーソルと見分ける。
const BEAT_BAND_BG: Color = Color::Rgb(55, 56, 48);
/// 名前を得られなかった note の表示。
const UNKNOWN_NAME: &str = "?";

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
    let resolution = match screen.kit_resolution() {
        Some(KitResolution::Waiting) => "  (catalog 照合中)",
        Some(KitResolution::Missing) => "  (catalog に無い kit)",
        None => "",
    };
    frame.render_widget(
        Paragraph::new(format!(
            "Pattern {}  Kit: {kit}{resolution}",
            screen.pattern_index()
        ))
        .style(base_style()),
        layout.kit,
    );
    if screen.notes().is_empty() {
        draw_empty(screen, frame, layout.matrix);
    } else {
        let label = label_width(layout.matrix.width);
        draw_steps(frame, layout.steps, label, screen.playhead());
        draw_matrix(screen, frame, layout.matrix, label);
    }
    let cursor = screen.cursor_note().map_or_else(String::new, |note| {
        format!(
            "Note {note} {}{}  Step {}/{}  {}",
            screen.note_name(note).unwrap_or(UNKNOWN_NAME),
            if screen.is_one_shot(note) {
                " (one-shot)"
            } else {
                ""
            },
            screen.cursor_step() + 1,
            DRUM_STEPS,
            match screen.cell_hit(note, screen.cursor_step()) {
                Some(hit) => format!(
                    "ON 長さ {}/{DRUM_STEPS} step  velocity {}",
                    hit.steps, hit.velocity
                ),
                None => "OFF".to_string(),
            }
        )
    });
    frame.render_widget(Paragraph::new(cursor).style(base_style()), layout.cursor);
    // 前置した回数は vim と同じく右端に出す。host の status とは重ねない。
    let count = screen
        .pending_count()
        .map_or_else(String::new, |count| format!(" {count}"));
    let [status_area, count_area] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(count.len() as u16)])
            .areas(layout.status);
    frame.render_widget(Paragraph::new(status).style(base_style()), status_area);
    frame.render_widget(
        Paragraph::new(count).style(base_style().fg(MONOKAI_YELLOW)),
        count_area,
    );
    frame.render_widget(
        Paragraph::new(
            "hjkl/←↓↑→:移動 Space:切替 -/+:長さ ,/.:velocity [/]:pattern t:kit Shift+P:再生/停止 a:先読み Ctrl+G:画面 q:終了 ?:help",
        )
        .style(base_style().fg(MONOKAI_GRAY)),
        layout.keys,
    );
    if screen.help_open() {
        help::draw_overlay(frame);
    }
}

/// step 列の残りを左列へ回す。狭い端末では note number だけになる。
fn label_width(matrix_width: u16) -> u16 {
    matrix_width
        .saturating_sub((DRUM_STEPS * STEP_WIDTH) as u16)
        .clamp(NOTE_WIDTH, LABEL_MAX_WIDTH)
}

/// (左列, step 列)。左列の右端 1 桁は空けて、切った名前と step 列を離す。
fn split_label(area: Rect, label: u16) -> (Rect, Rect) {
    let columns = Layout::horizontal([
        Constraint::Length(label.saturating_sub(1)),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(area);
    (columns[0], columns[2])
}

/// 2・4 拍目の step だけ背景を塗る。
fn step_style(step: usize) -> Style {
    if (step / STEPS_PER_BEAT) % 2 == 1 {
        base_style().bg(BEAT_BAND_BG)
    } else {
        base_style()
    }
}

fn draw_empty(screen: &DrumSequencerScreen, frame: &mut Frame<'_>, area: Rect) {
    let message = if screen.kit_name().is_none() {
        "t で Drum kit を選択してください。"
    } else if screen.kit_resolution() == Some(KitResolution::Waiting) {
        "保存した kit を patch catalog と照合しています。"
    } else if screen.kit_resolution() == Some(KitResolution::Missing) {
        "保存した kit が patch catalog にありません。入力は残しています。t で別の kit を選択してください。"
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

/// 演奏位置の step は番号を反転して示す。
fn draw_steps(frame: &mut Frame<'_>, area: Rect, label: u16, playhead: Option<usize>) {
    let (label_area, steps_area) = split_label(area, label);
    frame.render_widget(Paragraph::new("Note").style(base_style()), label_area);
    let spans: Vec<_> = (0..DRUM_STEPS)
        .map(|step| {
            let text = format!("{:^STEP_WIDTH$}", step + 1);
            if playhead == Some(step) {
                Span::styled(
                    text,
                    step_style(step)
                        .fg(MONOKAI_YELLOW)
                        .add_modifier(Modifier::REVERSED),
                )
            } else {
                Span::styled(text, step_style(step))
            }
        })
        .collect();
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(base_style()),
        steps_area,
    );
}

fn draw_matrix(screen: &mut DrumSequencerScreen, frame: &mut Frame<'_>, area: Rect, label: u16) {
    let (label_area, steps_area) = split_label(area, label);
    let rows: Vec<usize> = screen
        .visible_rows(usize::from(area.height))
        .map(|display| screen.row_at_display(display))
        .collect();
    let labels: Vec<Line<'static>> = rows
        .iter()
        .map(|&row| {
            let note = screen.notes()[row];
            let name = screen.note_name(note).unwrap_or(UNKNOWN_NAME);
            Line::from(format!("{note:>3} {name}"))
        })
        .collect();
    // 長い名前は左列の幅で切れ、step 列へはみ出さない。
    frame.render_widget(Paragraph::new(labels).style(base_style()), label_area);
    let lines: Vec<Line<'static>> = rows
        .into_iter()
        .map(|row| {
            let note = screen.notes()[row];
            let mut spans = Vec::with_capacity(DRUM_STEPS);
            // 直前の打点が鳴っている step の終わり（この step は含まない）。
            let mut sounding_until = 0;
            for step in 0..DRUM_STEPS {
                let length = screen.cell_length(note, step);
                let on = length.is_some();
                if let Some(steps) = length {
                    sounding_until = step + usize::from(steps);
                }
                let selected = row == screen.cursor_row && step == screen.cursor_step();
                let mark = match (on, step < sounding_until) {
                    (true, _) => 'x',
                    (false, true) => '-',
                    (false, false) => '.',
                };
                let color = match (mark, screen.playhead() == Some(step)) {
                    ('x', true) => MONOKAI_YELLOW,
                    ('x' | '-', _) => MONOKAI_GREEN,
                    _ => MONOKAI_GRAY,
                };
                let mut style = step_style(step).fg(color);
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
    frame.render_widget(Paragraph::new(lines).style(base_style()), steps_area);
}

#[cfg(test)]
mod tests;
