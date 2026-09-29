//! notepad 画面の描画。
//!
//! どの主要画面を描くかの振り分けは app 側（`tui::ui`）が行い、ここは
//! notepad 画面ひとつだけを描く。

mod help;
mod overlay;
mod status;
mod syntax;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use cmrt_tui_core::sound_check_guide::SoundCheckGuidePresentation;
use cmrt_tui_core::theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_YELLOW};

use crate::render_queue::TuiRenderJobStatus;
use crate::{Mode, NotepadScreen, PlayState, TuiRenderStatus};
use status::notepad_mode_title;
use status::{
    base_style, keybind_text, normal_status_text, render_status_color, render_status_text,
    status_text, visible_list_page_size,
};

// LIST_HIGHLIGHT_SYMBOL は画面横断で共有するため `cmrt-tui-core` へ切り出した。
use cmrt_tui_core::status::LIST_HIGHLIGHT_SYMBOL;

const LIST_HIGHLIGHT_WIDTH: u16 = 2;
/// 経過秒の表示上限。印の幅（4 セル）に収める。
const MAX_SHOWN_RENDER_SECS: u64 = 999;

/// 行頭の印（幅 4 セル）。順番待ちは `待ち`、render 中は開始からの経過秒。
pub(crate) fn cache_marker(cached: bool, render_status: Option<TuiRenderJobStatus>) -> String {
    if cached {
        return "♪   ".to_string();
    }
    match render_status {
        Some(TuiRenderJobStatus::Pending) => "待ち".to_string(),
        Some(TuiRenderJobStatus::Running { elapsed }) => {
            format!("{:>3}s", elapsed.as_secs().min(MAX_SHOWN_RENDER_SECS))
        }
        None => "    ".to_string(),
    }
}

pub(crate) fn mml_cache_hit(
    cache: &std::collections::HashMap<String, Vec<f32>>,
    disk_hashes: &std::collections::HashSet<u64>,
    mml: &str,
) -> bool {
    let mml = mml.trim();
    !mml.is_empty()
        && (cache.contains_key(mml) || disk_hashes.contains(&cmrt_history::daw_cache_mml_hash(mml)))
}

pub fn draw(app: &mut NotepadScreen<'_>, f: &mut Frame) {
    // play_state を一度だけロックしてスナップショットを取り、
    // status_text と status_color を同じ状態から導出する（二重ロック・状態不整合を防ぐ）。
    let play_state = app.playback.session.play_state_snapshot();
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
        if mode == Mode::Normal
            && app.sound_check_guide.presentation() == SoundCheckGuidePresentation::Overlay
        {
            cmrt_tui_core::ui::draw_sound_check_guide_overlay(
                f,
                f.area(),
                crate::NOTEPAD_SOUND_CHECK_GUIDE_MESSAGE,
            );
        }
    }
}

fn highlighted_spans(line: &str, style: Style, is_cursor: bool) -> Vec<Span<'static>> {
    syntax::highlight(line)
        .into_iter()
        .map(|(range, kind)| {
            let style = match kind {
                Some(kind) if is_cursor => cursor_highlight_style(base_style().fg(kind.color())),
                Some(kind) => style.fg(kind.color()),
                None => style,
            };
            Span::styled(line[range].to_string(), style)
        })
        .collect()
}

fn status_color(play_state: &PlayState) -> Color {
    status::status_color(play_state)
}

fn draw_normal(
    app: &mut NotepadScreen<'_>,
    f: &mut Frame,
    play_state: &PlayState,
    status_color: Color,
    mode: Mode,
) {
    let is_insert = mode == Mode::Insert;
    let cursor = app.editor.cursor;
    let mut status = normal_status_text(&mode, play_state);
    if app.patch_select_open_pending() {
        status.push_str("  ⏳ 音色一覧を読み込み中（終わったら音色選択を開きます）");
    }
    let render_status_snapshot = app.render_status_snapshot();
    let render_status = render_status_text(&render_status_snapshot);
    let render_status_color = render_status_color(&render_status_snapshot);
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
    app.editor.page_size = visible_list_page_size(list_area);
    let visible_height = usize::from(list_area.height.saturating_sub(2));
    let cache = app.audio.cache.lock().unwrap();
    let disk_hashes = app.audio.known_disk_hashes.lock().unwrap();

    let items: Vec<ListItem> = app
        .editor
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let style = if i == cursor {
                cursor_highlight_style(base_style())
            } else {
                base_style()
            };
            let cached = mml_cache_hit(&cache, &disk_hashes, line);
            let render_status = (!cached)
                .then(|| app.render_job_status_for_mml(line))
                .flatten();
            // INSERT 時のカーソル行は textarea で別描画するため、
            // List 側は空文字にして重なり表示を防ぐ。
            let content = if is_insert && i == cursor {
                String::new()
            } else {
                line.clone()
            };
            let mut spans = vec![Span::styled(cache_marker(cached, render_status), style)];
            // 選択行は必ず表示されるので、表示中の行はカーソルから表示高さ未満の距離にある。
            if i.abs_diff(cursor) < visible_height {
                spans.extend(highlighted_spans(&content, style, i == cursor));
            } else {
                spans.push(Span::styled(content, style));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    f.render_stateful_widget(
        List::new(items)
            .style(base_style())
            // fg を持たせると、選択行の span ごとのシンタックスハイライトを上書きしてしまう。
            .highlight_style(cursor_highlight_style(Style::default()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(notepad_mode_title(&mode))
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_CYAN)),
            )
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        list_area,
        &mut app.editor.list_state,
    );

    // INSERTモード時は、カーソル行にインラインで textarea を描画する。
    // List ウィジェットは Borders::ALL を持つため、内側の開始は +1 ずつオフセットする。
    if is_insert {
        let offset = app.editor.list_state.offset();
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
                f.render_widget(&app.editor.textarea, textarea_area);
            }
        }
    }

    f.render_widget(
        Paragraph::new(status).style(base_style().fg(status_color)),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(render_status).style(base_style().fg(render_status_color)),
        chunks[2],
    );
    if mode == Mode::Normal
        && app.sound_check_guide.presentation() == SoundCheckGuidePresentation::Footer
    {
        f.render_widget(
            Paragraph::new(Span::styled(
                crate::NOTEPAD_SOUND_CHECK_GUIDE_MESSAGE,
                base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
            ))
            .style(base_style()),
            chunks[3],
        );
    } else {
        f.render_widget(Paragraph::new(keybinds).style(base_style()), chunks[3]);
    }
}

#[cfg(test)]
mod tests;
