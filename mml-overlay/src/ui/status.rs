//! 入力欄の下の 1 行。左に「いま何が鳴っているか」、右にキー割り当てを出す。

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

use cmrt_tui_core::theme::{
    MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PINK, MONOKAI_YELLOW,
};

use crate::{line_play::LineStatus, MmlOverlay};

const KEY_HINTS: &str = "^T音色 ^O履歴 ^Space再演奏 Esc閉じる ";
/// キー割り当ての表示に要る幅（全角は 2 桁ぶん）。
const KEY_HINTS_WIDTH: u16 = 38;

pub(super) fn draw(overlay: &MmlOverlay<'_>, frame: &mut Frame<'_>, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(KEY_HINTS_WIDTH)])
        .split(area);

    let (label, color) = sounding_label(overlay);
    frame.render_widget(
        Paragraph::new(format!(" {label}")).style(Style::default().fg(color)),
        chunks[0],
    );
    if chunks.len() > 1 {
        frame.render_widget(
            Paragraph::new(KEY_HINTS)
                .style(Style::default().fg(MONOKAI_GRAY))
                .alignment(Alignment::Right),
            chunks[1],
        );
    }
}

/// 打鍵で鳴らした音が優先。鳴っていなければ直近の行演奏の結果を出す。
///
/// chord として解釈されたかどうかは、打鍵の音でも行の演奏でも色とラベルで
/// 見分けられるようにする。同じ入力がどちらへ倒れたのかがその場で分かることが、
/// この画面の目的の 1 つのため。
fn sounding_label(overlay: &MmlOverlay<'_>) -> (String, Color) {
    if !overlay.sounding().is_empty() {
        let names = overlay
            .sounding()
            .iter()
            .map(|pitch| note_name(*pitch))
            .collect::<Vec<_>>();
        let sounding = names.join(" ");
        return if overlay.sounding_from_chord() {
            (format!("CHORD ♪ {sounding}"), MONOKAI_GREEN)
        } else {
            (format!("♪ {sounding}"), MONOKAI_YELLOW)
        };
    }
    match overlay.line_status() {
        LineStatus::Idle => ("-".to_string(), MONOKAI_GRAY),
        LineStatus::Played {
            from_chord: true,
            note_count,
        } => (format!("CHORD {note_count}音"), MONOKAI_GREEN),
        LineStatus::Played {
            from_chord: false,
            note_count,
        } => (format!("MML {note_count}音"), MONOKAI_CYAN),
        LineStatus::Error(error) => (error.clone(), MONOKAI_PINK),
    }
}

/// オクターブは MML の数え方（C5 = 60）に合わせる。
fn note_name(pitch: u8) -> String {
    const NAMES: [&str; 12] = [
        "c", "c+", "d", "d+", "e", "f", "f+", "g", "g+", "a", "a+", "b",
    ];
    format!("{}{}", NAMES[usize::from(pitch) % 12], pitch / 12)
}

#[cfg(test)]
mod tests;
