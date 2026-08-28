//! MML 入力オーバーレイの描画。

mod chord_transfer;
mod history_select;
mod loading;
mod patch_select;
mod play_settings;
mod status;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear},
    Frame,
};
use ratatui_textarea::TextArea;

use cmrt_tui_core::{
    text_input::single_line_textarea_cursor_position,
    theme::{MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY},
    ui::centered_rect_with_size,
};

use crate::{MmlOverlay, MmlOverlayInputMode, MmlOverlaySenderStatus};

const PLACEHOLDER: &str = "1行1フレーズ。上下キーでその行を演奏";
const SINGLE_LINE_PLACEHOLDER: &str = "MMLを入力。Enterで確定";
/// 複数行モードの入力欄に見せる行数。これを超えた行は入力欄の中でスクロールする。
const INPUT_ROWS: u16 = 8;
/// 1 行モードの入力欄に見せる行数。書き戻す先が 1 か所なので 1 行しか要らない。
const SINGLE_LINE_INPUT_ROWS: u16 = 1;
const OVERLAY_MAX_WIDTH: u16 = 72;

/// 入力欄の枠(2行) + 入力行 + 状態行(1行) + chord ヒント(立っているときだけ 1 行)。
fn overlay_height(input_mode: MmlOverlayInputMode, hint_rows: u16) -> u16 {
    input_rows(input_mode) + 3 + hint_rows
}

fn input_rows(input_mode: MmlOverlayInputMode) -> u16 {
    match input_mode {
        MmlOverlayInputMode::MultiLine => INPUT_ROWS,
        MmlOverlayInputMode::SingleLine => SINGLE_LINE_INPUT_ROWS,
    }
}

fn placeholder(input_mode: MmlOverlayInputMode) -> &'static str {
    match input_mode {
        MmlOverlayInputMode::MultiLine => PLACEHOLDER,
        MmlOverlayInputMode::SingleLine => SINGLE_LINE_PLACEHOLDER,
    }
}

pub fn draw(overlay: &MmlOverlay<'_>, frame: &mut Frame<'_>) {
    draw_with_status(overlay, &MmlOverlaySenderStatus::default(), frame);
}

pub fn draw_with_status(
    overlay: &MmlOverlay<'_>,
    sender_status: &MmlOverlaySenderStatus,
    frame: &mut Frame<'_>,
) {
    let area = frame.area();
    let width = area.width.saturating_sub(2).min(OVERLAY_MAX_WIDTH);
    let hint_rows = chord_transfer::hint_rows(overlay);
    let height = overlay_height(overlay.input_mode(), hint_rows).min(area.height);
    let overlay_area = centered_rect_with_size(width, height, area);
    frame.render_widget(Clear, overlay_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(hint_rows),
        ])
        .split(overlay_area);
    draw_input(overlay, frame, chunks[0]);
    if chunks.len() > 1 {
        status::draw(overlay, frame, chunks[1]);
    }
    if chunks.len() > 2 && hint_rows > 0 {
        chord_transfer::draw_hint(frame, chunks[2]);
    }

    // 音色選択と履歴は入力欄へ重ねて出す。入力欄より後に描くこと。
    if let Some(select) = overlay.patch_select() {
        patch_select::draw(select, frame);
    }
    if let Some(select) = overlay.history_select() {
        history_select::draw(select, frame);
    }
    // 演奏設定は音色選択の最中にも開ける最も手前のモーダルなので、最後に描く。
    if let Some(select) = overlay.play_settings_select() {
        play_settings::draw(select, frame);
    }
    // 確定ダイアログはさらに手前。開いている間は他のモーダルが開くことは無いが、
    // 描く順としては最後に置く（最後の砦なので何にも隠されない）。
    if let Some(confirm) = overlay.chord_transfer_confirm() {
        chord_transfer::draw(confirm, frame);
    }
    loading::draw(sender_status, frame);
}

fn draw_input(overlay: &MmlOverlay<'_>, frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        &build_input_widget(
            overlay.textarea(),
            title(overlay),
            placeholder(overlay.input_mode()),
        ),
        area,
    );
    frame.set_cursor_position(single_line_textarea_cursor_position(
        area,
        overlay.textarea(),
    ));
}

/// 描画用の複製へ枠と placeholder を付ける。
///
/// 持ち続けている `TextArea` はカーソル位置と編集履歴を持つので、フレームごとの
/// 見た目だけをここで足して本体には触らない。
fn build_input_widget<'a>(
    textarea: &TextArea<'a>,
    title: String,
    placeholder: &str,
) -> TextArea<'a> {
    let mut widget = textarea.clone();
    widget.set_placeholder_text(placeholder);
    widget.set_placeholder_style(Style::default().fg(MONOKAI_GRAY).bg(MONOKAI_BG));
    widget.set_block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG))
            .border_style(Style::default().fg(MONOKAI_CYAN)),
    );
    widget
}

/// 入力欄の枠のタイトル。音色は入力欄のテキストに現れないので、ここへ出す。
fn title(overlay: &MmlOverlay<'_>) -> String {
    match overlay.patch() {
        Some(patch) => format!(" MML [{patch}] "),
        None => " MML [既定音色] ".to_string(),
    }
}

#[cfg(test)]
mod tests;
