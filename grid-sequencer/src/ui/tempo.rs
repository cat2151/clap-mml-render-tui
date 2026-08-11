use ratatui::{
    layout::Alignment,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use cmrt_tui_core::{status::base_style, theme::MONOKAI_GREEN, ui::centered_rect_with_size};

use crate::GridSequencerScreen;

pub(super) fn draw_overlay(frame: &mut Frame<'_>, screen: &GridSequencerScreen) {
    let Some(input) = screen.bpm_input.as_ref() else {
        return;
    };
    let mut lines = vec![
        Line::from(format!(
            "現在: {} {}",
            screen.bpm(),
            screen.bpm_mode().label()
        )),
        Line::from(format!("BPM (20〜300): {}_", input.buffer())),
        Line::from("Enter:確定  A:自動BPM  Esc:cancel"),
    ];
    if let Some(error) = input.error() {
        lines.push(Line::from(error.to_string()));
    }
    let height = (lines.len() as u16).saturating_add(2);
    let area = centered_rect_with_size(54.min(frame.area().width), height, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" BPM設定 ")
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_GREEN)),
            ),
        area,
    );
}
