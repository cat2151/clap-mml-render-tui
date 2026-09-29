//! 重い音色の試聴の確認・待機ダイアログ。音色選択の上に重ねる。

use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::patches::patch_stem;
use cmrt_tui_core::theme::{MONOKAI_PINK, MONOKAI_YELLOW};

use crate::input::HeavyPreview;

use super::super::status::base_style;

const DIALOG_WIDTH: u16 = 64;
const DIALOG_HEIGHT: u16 = 8;

/// 自動試聴を止めていることの案内。音色選択の status 行へ足す。
pub(super) const AUTO_PREVIEW_SKIPPED_NOTE: &str =
    "  重い音色のため自動試聴しません（Space で試聴）";

pub(super) fn draw(preview: &HeavyPreview, f: &mut Frame) {
    let (title, lines) = match preview {
        HeavyPreview::Confirm {
            patch_name,
            sample_bytes,
        } => (
            " 重い音色の試聴 ",
            vec![
                Line::from(patch_stem(patch_name).to_string()),
                Line::from(format!(
                    "sample {}MB を読み込むため、鳴るまで時間がかかります。",
                    sample_bytes / 1_000_000
                )),
                Line::from("始まると途中で止められず、終わるまで操作できません。"),
                Line::from(""),
                Line::from("y/Enter: 試聴する    n/Esc: やめる"),
            ],
        ),
        HeavyPreview::Waiting {
            patch_name,
            started,
            ..
        } => (
            " 重い音色を読み込み中 ",
            vec![
                Line::from(patch_stem(patch_name).to_string()),
                Line::from(format!("{}秒経過", started.elapsed().as_secs())),
                Line::from(""),
                Line::from("鳴り始めたらこの表示は閉じます。止められません。"),
            ],
        ),
    };
    let area = centered(f.area());
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .style(base_style().fg(MONOKAI_YELLOW))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_PINK)),
            ),
        area,
    );
}

fn centered(area: Rect) -> Rect {
    let width = DIALOG_WIDTH.min(area.width);
    let height = DIALOG_HEIGHT.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}
