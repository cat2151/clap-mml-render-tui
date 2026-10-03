//! SMF 素材の描画。読み込み overlay（`O`）と、素材の間に MML 欄へ出す file 名。

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Clear,
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_PINK},
    ui::centered_rect_with_size,
};

use crate::GuitarArticulationScreen;

pub(super) const INPUT_TITLE: &str = " SMF を読む  Enter:読み込み  Esc:閉じる ";
const INPUT_PLACEHOLDER: &str = "SMF のパス（\"\" 付きのままでよい）";
/// overlay の幅の上限（枠込み）。
const INPUT_MAX_WIDTH: u16 = 90;
/// overlay の高さ（枠込みの 1 行入力）。
const INPUT_HEIGHT: u16 = 3;

/// タイトルに足す ` [SMF]`（単音化 on なら ` [SMF] [top]`）。SMF 素材でなければ空。
pub(super) fn title_flags(screen: &GuitarArticulationScreen) -> String {
    match (screen.smf_material_name(), screen.smf_top_note()) {
        (None, _) => String::new(),
        (Some(_), false) => " [SMF]".to_string(),
        (Some(_), true) => " [SMF] [top]".to_string(),
    }
}

/// 読み込み overlay。閉じていれば何もしない。
pub(super) fn draw_input_overlay(f: &mut Frame<'_>, screen: &GuitarArticulationScreen) {
    let Some(textarea) = screen.smf_input() else {
        return;
    };
    let area = input_rect(f.area());
    let value = cmrt_tui_core::text_input::textarea_value(textarea);
    let border = if screen.error.is_some() {
        MONOKAI_PINK
    } else {
        MONOKAI_CYAN
    };
    let widget = cmrt_tui_core::text_input::build_query_textarea_widget(
        textarea,
        &value,
        INPUT_TITLE,
        INPUT_PLACEHOLDER,
        border,
    );
    f.render_widget(Clear, area);
    f.render_widget(&widget, area);
    // 点滅する縦線カーソルを入力欄へ置く（app 側の `uses_textarea_cursor` と対）。
    f.set_cursor_position(
        cmrt_tui_core::text_input::single_line_textarea_cursor_position(area, textarea),
    );
}

pub(super) fn input_rect(area: Rect) -> Rect {
    let width = area.width.saturating_sub(4).min(INPUT_MAX_WIDTH);
    centered_rect_with_size(width, INPUT_HEIGHT, area)
}

/// MML 欄の中身。SMF 素材なら灰色の `SMF: <file 名>`、MML が空なら案内、それ以外は MML。
pub(super) fn mml_field_line(screen: &GuitarArticulationScreen) -> Line<'static> {
    if let Some(name) = screen.smf_material_name() {
        return Line::from(Span::styled(
            format!("SMF: {name}"),
            base_style().fg(MONOKAI_GRAY),
        ));
    }
    if screen.mml().is_empty() {
        Line::from(Span::styled(
            "(i キーで MML 入力開始)",
            base_style().fg(MONOKAI_GRAY),
        ))
    } else {
        Line::from(screen.mml().to_string())
    }
}
