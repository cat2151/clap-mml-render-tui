//! MML 入力オーバーレイの描画。

mod patch_select;

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    text_input::{
        build_query_textarea_widget, single_line_textarea_cursor_position, textarea_value,
    },
    theme::{MONOKAI_CYAN, MONOKAI_GRAY},
    ui::centered_rect_with_size,
};

use crate::MmlOverlay;

const PLACEHOLDER: &str = "cde";
/// 入力欄の枠(2行) + 入力行(1行) + 発音表示(1行)。
const OVERLAY_HEIGHT: u16 = 4;
const OVERLAY_MAX_WIDTH: u16 = 64;

pub fn draw(overlay: &MmlOverlay<'_>, frame: &mut Frame<'_>) {
    let area = frame.area();
    let width = area.width.saturating_sub(2).min(OVERLAY_MAX_WIDTH);
    let overlay_area = centered_rect_with_size(width, OVERLAY_HEIGHT.min(area.height), area);
    frame.render_widget(Clear, overlay_area);

    let input_area = Rect {
        height: overlay_area.height.saturating_sub(1),
        ..overlay_area
    };
    let value = textarea_value(overlay.textarea());
    frame.render_widget(
        &build_query_textarea_widget(
            overlay.textarea(),
            &value,
            title(overlay),
            PLACEHOLDER,
            MONOKAI_CYAN,
        ),
        input_area,
    );
    frame.set_cursor_position(single_line_textarea_cursor_position(
        input_area,
        overlay.textarea(),
    ));

    if overlay_area.height >= OVERLAY_HEIGHT {
        let status_area = Rect {
            y: overlay_area.y + overlay_area.height - 1,
            height: 1,
            ..overlay_area
        };
        frame.render_widget(
            Paragraph::new(sounding_label(overlay.sounding()))
                .style(Style::default().fg(MONOKAI_GRAY)),
            status_area,
        );
    }

    // 音色選択は入力欄へ重ねて出す。入力欄より後に描くこと。
    if let Some(select) = overlay.patch_select() {
        patch_select::draw(select, frame);
    }
}

/// 入力欄の枠のタイトル。行頭 JSON は横に長く、幅の狭い入力欄では
/// スクロールアウトして見えなくなるため、いま何の音色かはここへ出す。
///
/// 表示は入力欄の中身から読み直す。手で JSON を書き換えたときも、
/// 実際に書かれている音色がそのまま出る。
fn title(overlay: &MmlOverlay<'_>) -> String {
    match crate::patch_json::patch_name(&overlay.value()) {
        Some(patch) => format!(" MML [{patch}]  Ctrl+T:音色  Esc:close "),
        None => " MML  Ctrl+T:音色  Esc:close ".to_string(),
    }
}

/// いま鳴っている音の表示。オクターブは MML の数え方（C5 = 60）に合わせる。
fn sounding_label(sounding: &[u8]) -> String {
    if sounding.is_empty() {
        return " sounding: -".to_string();
    }
    let names = sounding
        .iter()
        .map(|pitch| note_name(*pitch))
        .collect::<Vec<_>>();
    format!(" sounding: {}", names.join(" "))
}

fn note_name(pitch: u8) -> String {
    const NAMES: [&str; 12] = [
        "c", "c+", "d", "d+", "e", "f", "f+", "g", "g+", "a", "a+", "b",
    ];
    format!("{}{}", NAMES[usize::from(pitch) % 12], pitch / 12)
}

#[cfg(test)]
mod tests;
