//! UI テーマ定義（TUI / DAW 共通）

use ratatui::style::{Color, Modifier, Style};

pub(crate) const MONOKAI_BG: Color = Color::Rgb(39, 40, 34);
pub(crate) const MONOKAI_FG: Color = Color::Rgb(248, 248, 242);
pub(crate) const MONOKAI_GRAY: Color = Color::Rgb(160, 160, 160);
pub(crate) const MONOKAI_PINK: Color = Color::Rgb(249, 38, 114);
pub(crate) const MONOKAI_YELLOW: Color = Color::Rgb(230, 219, 116);
pub(crate) const MONOKAI_GREEN: Color = Color::Rgb(166, 226, 46);
pub(crate) const MONOKAI_CYAN: Color = Color::Rgb(102, 217, 239);
pub(crate) const MONOKAI_PURPLE: Color = Color::Rgb(174, 129, 255);
pub(crate) const MONOKAI_CURSOR_BG: Color = Color::Rgb(73, 72, 62);
const MONOKAI_CURSOR_BG_ALT: Color = Color::Rgb(96, 96, 96);

fn color_distance_sq(lhs: Color, rhs: Color) -> Option<u32> {
    match (lhs, rhs) {
        (Color::Rgb(lr, lg, lb), Color::Rgb(rr, rg, rb)) => {
            let dr = lr.abs_diff(rr) as u32;
            let dg = lg.abs_diff(rg) as u32;
            let db = lb.abs_diff(rb) as u32;
            Some(dr * dr + dg * dg + db * db)
        }
        _ => None,
    }
}

pub(crate) fn cursor_highlight_bg(fg: Color) -> Color {
    let primary = MONOKAI_CURSOR_BG;
    let fallback = MONOKAI_CURSOR_BG_ALT;
    [primary, fallback]
        .into_iter()
        .max_by_key(|bg| color_distance_sq(fg, *bg).unwrap_or(0))
        .unwrap_or(primary)
}

pub(crate) fn blinking_cursor_style(style: Style) -> Style {
    let fg = style.fg.unwrap_or(MONOKAI_FG);
    style
        .bg(cursor_highlight_bg(fg))
        .add_modifier(Modifier::BOLD)
}
