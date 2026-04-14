//! TUI 描画

mod help;
mod overlay;
mod status;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use super::{Mode, PlayState, TuiApp};
use crate::ui_theme::{cursor_highlight_style, MONOKAI_CYAN};
use status::notepad_mode_title;
use status::{
    base_style, keybind_text, normal_status_text, parallel_render_status_color,
    parallel_render_status_text, status_text, visible_list_page_size,
};

const LIST_HIGHLIGHT_SYMBOL: &str = "▶ ";
const LIST_HIGHLIGHT_WIDTH: u16 = 2;

pub(super) fn draw(app: &mut TuiApp<'_>, f: &mut Frame) {
    // play_state を一度だけロックしてスナップショットを取り、
    // status_text と status_color を同じ状態から導出する（二重ロック・状態不整合を防ぐ）。
    let play_state = app.play_state.lock().unwrap().clone();
    let mode = app.mode;
    let help_origin = app.help_origin;
    let status = status_text(&mode, &play_state);
    let status_color = status_color(&play_state);

    if mode == Mode::Help {
        match help_origin {
            Mode::PatchSelect => {
                draw_normal(app, f, &play_state, status_color, help_origin);
                let overlay_status = status_text(&help_origin, &play_state);
                overlay::draw_patch_select(app, f, &overlay_status, status_color, help_origin);
            }
            Mode::NotepadHistory => {
                draw_normal(app, f, &play_state, status_color, help_origin);
                let overlay_status = status_text(&help_origin, &play_state);
                overlay::draw_notepad_history(app, f, &overlay_status, status_color, help_origin);
            }
            Mode::PatchPhrase => {
                draw_normal(app, f, &play_state, status_color, help_origin);
                let overlay_status = status_text(&help_origin, &play_state);
                overlay::draw_patch_phrase(app, f, &overlay_status, status_color, help_origin);
            }
            _ => draw_normal(app, f, &play_state, status_color, mode),
        }
        help::draw_help(f, help_origin);
    } else if mode == Mode::PatchSelect {
        draw_normal(app, f, &play_state, status_color, mode);
        overlay::draw_patch_select(app, f, &status, status_color, mode);
    } else if mode == Mode::NotepadHistory {
        draw_normal(app, f, &play_state, status_color, mode);
        overlay::draw_notepad_history(app, f, &status, status_color, mode);
    } else if mode == Mode::NotepadHistoryGuide {
        draw_normal(app, f, &play_state, status_color, mode);
        overlay::draw_notepad_history_guide(f);
    } else if mode == Mode::PatchPhrase {
        draw_normal(app, f, &play_state, status_color, mode);
        overlay::draw_patch_phrase(app, f, &status, status_color, mode);
    } else {
        draw_normal(app, f, &play_state, status_color, mode);
    }
}

fn status_color(play_state: &PlayState) -> Color {
    status::status_color(play_state)
}

fn draw_normal(
    app: &mut TuiApp<'_>,
    f: &mut Frame,
    play_state: &PlayState,
    status_color: Color,
    mode: Mode,
) {
    let is_insert = mode == Mode::Insert;
    let cursor = app.cursor;
    let status = normal_status_text(&mode, play_state);
    let active_parallel_render_count = app.active_parallel_render_count();
    let parallel_render_status = parallel_render_status_text(active_parallel_render_count);
    let parallel_render_status_color = parallel_render_status_color(active_parallel_render_count);
    let keybinds = keybind_text(&mode);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());
    let list_area = chunks[0];
    app.normal_page_size = visible_list_page_size(list_area);

    let items: Vec<ListItem> = app
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let style = if i == cursor {
                cursor_highlight_style(base_style())
            } else {
                base_style()
            };
            // INSERT 時のカーソル行は textarea で別描画するため、
            // List 側は空文字にして重なり表示を防ぐ。
            let content = if is_insert && i == cursor {
                String::new()
            } else {
                line.clone()
            };
            ListItem::new(Line::from(Span::styled(content, style)))
        })
        .collect();

    f.render_stateful_widget(
        List::new(items)
            .style(base_style())
            .highlight_style(cursor_highlight_style(base_style()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(notepad_mode_title(&mode))
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_CYAN)),
            )
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        list_area,
        &mut app.list_state,
    );

    // INSERTモード時は、カーソル行にインラインで textarea を描画する。
    // List ウィジェットは Borders::ALL を持つため、内側の開始は +1 ずつオフセットする。
    if is_insert {
        let offset = app.list_state.offset();
        if cursor >= offset {
            let row_in_visible = (cursor - offset) as u16;
            let inner_top = list_area.y + 1; // 上ボーダーの内側（1行分）
            let inner_bottom = list_area.y + list_area.height.saturating_sub(1); // 下ボーダーの位置
            let textarea_y = inner_top + row_in_visible;
            if textarea_y < inner_bottom {
                let textarea_area = Rect {
                    x: list_area.x + 1 + LIST_HIGHLIGHT_WIDTH,
                    y: textarea_y,
                    width: list_area.width.saturating_sub(2 + LIST_HIGHLIGHT_WIDTH),
                    height: 1,
                };
                f.render_widget(Clear, textarea_area);
                f.render_widget(&app.textarea, textarea_area);
            }
        }
    }

    f.render_widget(
        Paragraph::new(status).style(base_style().fg(status_color)),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(parallel_render_status).style(base_style().fg(parallel_render_status_color)),
        chunks[2],
    );
    f.render_widget(Paragraph::new(keybinds).style(base_style()), chunks[3]);
}

#[cfg(test)]
#[path = "../tests/tui_ui.rs"]
mod tests;
