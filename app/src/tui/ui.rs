//! TUI 描画

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use super::{Mode, PatchPhrasePane, PlayState, TuiApp};

const LIST_HIGHLIGHT_SYMBOL: &str = "▶ ";
const LIST_HIGHLIGHT_WIDTH: u16 = 2;
const MONOKAI_BG: Color = Color::Rgb(39, 40, 34);
const MONOKAI_FG: Color = Color::Rgb(248, 248, 242);
const MONOKAI_GRAY: Color = Color::Rgb(160, 160, 160);
const MONOKAI_YELLOW: Color = Color::Rgb(230, 219, 116);
const MONOKAI_GREEN: Color = Color::Rgb(166, 226, 46);
const MONOKAI_CYAN: Color = Color::Rgb(102, 217, 239);
const MONOKAI_PURPLE: Color = Color::Rgb(174, 129, 255);

fn base_style() -> Style {
    Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG)
}

pub(super) fn draw(app: &mut TuiApp<'_>, f: &mut Frame) {
    // play_state を一度だけロックしてスナップショットを取り、
    // status_text と status_color を同じ状態から導出する（二重ロック・状態不整合を防ぐ）。
    let play_state = app.play_state.lock().unwrap().clone();
    let status = status_text(app, &play_state);
    let status_color = status_color(&play_state);

    if app.mode == Mode::Help {
        draw_normal(app, f, &play_state, status_color);
        draw_help(f);
    } else if app.mode == Mode::PatchSelect {
        draw_normal(app, f, &play_state, status_color);
        draw_patch_select(app, f, &status, status_color);
    } else if app.mode == Mode::PatchPhrase {
        draw_patch_phrase(app, f, &status, status_color);
    } else {
        draw_normal(app, f, &play_state, status_color);
    }
}

fn status_color(play_state: &PlayState) -> Color {
    match play_state {
        PlayState::Err(_) => Color::Red,
        PlayState::Running(_) => MONOKAI_PURPLE,
        PlayState::Playing(_) => MONOKAI_YELLOW,
        PlayState::Done(_) => MONOKAI_GREEN,
        PlayState::Idle => MONOKAI_CYAN,
    }
}

fn play_status_suffix(play_state: &PlayState) -> String {
    match play_state {
        PlayState::Idle => "".to_string(),
        PlayState::Running(mml) => format!("  ⚙ レンダリング中: {}", mml),
        PlayState::Playing(msg) => format!("  ▶ 演奏中: {}", msg),
        PlayState::Done(msg) => format!("  ✓ {}", msg),
        PlayState::Err(msg) => format!("  ✗ {}", msg),
    }
}

fn normal_status_text(app: &TuiApp<'_>, play_state: &PlayState) -> String {
    let mode = match app.mode {
        Mode::Insert => "INSERT",
        Mode::Help => "HELP",
        _ => "NORMAL",
    };
    format!("{mode}{}", play_status_suffix(play_state))
}

fn keybind_text(mode: &Mode) -> &'static str {
    match mode {
        Mode::Normal => {
            "q:quit  ?:help  i:insert  r:ランダム音色  t:音色  p:phrase  j/k  Enter/Space  d:DAW"
        }
        Mode::Insert => "ESC:確定→NORMAL  Enter:確定→次行",
        Mode::PatchSelect => {
            "Enter:決定  ESC:キャンセル  j/k・↑↓:移動  文字入力:フィルタ  Space:AND条件"
        }
        Mode::PatchPhrase => {
            "j/k:再生移動  h/l:ペイン移動  Space/Enter:再生  i:編集  f:お気に入り  ESC:戻る"
        }
        Mode::Help => "ESC:キャンセル",
    }
}

fn status_text(app: &TuiApp<'_>, play_state: &PlayState) -> String {
    let play_str = play_status_suffix(play_state);
    match app.mode {
        Mode::Normal | Mode::Insert | Mode::Help => normal_status_text(app, play_state),
        Mode::PatchSelect => format!("音色選択{}", play_str),
        Mode::PatchPhrase => format!("patch phrase{}", play_str),
    }
}

fn draw_patch_select(app: &mut TuiApp<'_>, f: &mut Frame, status: &str, status_color: Color) {
    let area = crate::ui_utils::centered_rect(70, 70, f.area());
    f.render_widget(Clear, area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(format!("> {}", app.patch_query))
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" 音色選択 - 検索 (space=AND) ")
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_YELLOW)),
            ),
        chunks[0],
    );

    let count_title = format!(
        " パッチ ({}/{}) ",
        app.patch_filtered.len(),
        app.patch_all.len()
    );
    let patch_items: Vec<ListItem> = app
        .patch_filtered
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == app.patch_cursor {
                base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD)
            } else {
                base_style()
            };
            ListItem::new(Span::styled(p.clone(), style))
        })
        .collect();

    f.render_stateful_widget(
        List::new(patch_items)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(count_title)
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_CYAN)),
            )
            .highlight_symbol("▶ "),
        chunks[1],
        &mut app.patch_list_state,
    );

    f.render_widget(
        Paragraph::new(status.to_string()).style(base_style().fg(status_color)),
        chunks[2],
    );
    f.render_widget(
        Paragraph::new(keybind_text(&app.mode)).style(base_style()),
        chunks[3],
    );
}

fn draw_normal(app: &mut TuiApp<'_>, f: &mut Frame, play_state: &PlayState, status_color: Color) {
    let is_insert = app.mode == Mode::Insert;
    let cursor = app.cursor;
    let status = normal_status_text(app, play_state);
    let keybinds = keybind_text(&app.mode);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());

    let items: Vec<ListItem> = app
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let style = if i == cursor {
                base_style().fg(MONOKAI_CYAN).add_modifier(Modifier::BOLD)
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
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(if is_insert { " [INSERT] " } else { "" })
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_CYAN)),
            )
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        chunks[0],
        &mut app.list_state,
    );

    // INSERTモード時は、カーソル行にインラインで textarea を描画する。
    // List ウィジェットは Borders::ALL を持つため、内側の開始は +1 ずつオフセットする。
    if is_insert {
        let list_area = chunks[0];
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
    f.render_widget(Paragraph::new(keybinds).style(base_style()), chunks[2]);
}

fn draw_patch_phrase(app: &mut TuiApp<'_>, f: &mut Frame, status: &str, status_color: Color) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let patch_name = app.patch_phrase_name.as_deref().unwrap_or("(unknown)");
    let history_items: Vec<ListItem> = app
        .patch_phrase_history_items()
        .into_iter()
        .enumerate()
        .map(|(i, phrase)| {
            let is_selected = app.patch_phrase_focus == PatchPhrasePane::History
                && i == app.patch_phrase_history_cursor;
            let style = if is_selected {
                base_style().fg(MONOKAI_CYAN).add_modifier(Modifier::BOLD)
            } else {
                base_style()
            };
            ListItem::new(Span::styled(phrase, style))
        })
        .collect();
    let favorite_items: Vec<ListItem> = app
        .patch_phrase_favorite_items()
        .into_iter()
        .enumerate()
        .map(|(i, phrase)| {
            let is_selected = app.patch_phrase_focus == PatchPhrasePane::Favorites
                && i == app.patch_phrase_favorites_cursor;
            let style = if is_selected {
                base_style().fg(MONOKAI_CYAN).add_modifier(Modifier::BOLD)
            } else {
                base_style()
            };
            ListItem::new(Span::styled(phrase, style))
        })
        .collect();

    let history_border = if app.patch_phrase_focus == PatchPhrasePane::History {
        base_style().fg(MONOKAI_CYAN)
    } else {
        base_style()
    };
    let favorites_border = if app.patch_phrase_focus == PatchPhrasePane::Favorites {
        base_style().fg(MONOKAI_CYAN)
    } else {
        base_style()
    };

    f.render_stateful_widget(
        List::new(history_items)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" History - {patch_name} "))
                    .style(base_style())
                    .border_style(history_border),
            )
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        panes[0],
        &mut app.patch_phrase_history_state,
    );
    f.render_stateful_widget(
        List::new(favorite_items)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Favorites ")
                    .style(base_style())
                    .border_style(favorites_border),
            )
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        panes[1],
        &mut app.patch_phrase_favorites_state,
    );

    f.render_widget(
        Paragraph::new(status.to_string()).style(base_style().fg(status_color)),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(keybind_text(&app.mode)).style(base_style()),
        chunks[2],
    );
}

fn draw_help(f: &mut Frame) {
    let area = crate::ui_utils::centered_rect(60, 80, f.area());
    f.render_widget(Clear, area);

    let help_lines = vec![
        Line::from(Span::styled(
            "NORMAL モード",
            base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
        )),
        Line::from("  j / ↓       : 下へ移動"),
        Line::from("  k / ↑       : 上へ移動"),
        Line::from("  H           : 先頭行へ移動"),
        Line::from("  M           : 中央行へ移動"),
        Line::from("  L           : 末尾行へ移動"),
        Line::from("  Enter/Space : 再生"),
        Line::from("  i           : INSERT モード"),
        Line::from("  o           : 次行に挿入 → INSERT"),
        Line::from("  r           : ランダム音色を挿入/置換して再生"),
        Line::from("  t           : 音色選択"),
        Line::from("  p           : patch phrase 画面"),
        Line::from("  d           : DAW モード"),
        Line::from("  K / ?       : ヘルプ (このページ)"),
        Line::from("  q           : 終了"),
        Line::from(""),
        Line::from(Span::styled(
            "INSERT モード",
            base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
        )),
        Line::from("  ESC   : 確定 → NORMAL (再生)"),
        Line::from("  Enter : 確定 → 次行挿入 → INSERT 継続"),
        Line::from("  Ctrl+C: コピー"),
        Line::from("  Ctrl+X: カット"),
        Line::from("  Ctrl+V: ペースト"),
        Line::from(""),
        Line::from(Span::styled(
            "音色選択モード",
            base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
        )),
        Line::from("  文字入力 : フィルタ (Space=AND条件)"),
        Line::from("  ↑↓      : リスト移動"),
        Line::from("  Enter   : 音色決定"),
        Line::from("  ESC     : キャンセル"),
        Line::from(""),
        Line::from(Span::styled(
            "patch phrase 画面",
            base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
        )),
        Line::from("  j / k       : 上下移動して再生"),
        Line::from("  h / l       : ペイン切替して再生"),
        Line::from("  Space/Enter : 現在行を再生"),
        Line::from("  i           : History行を編集"),
        Line::from("  f     : 現在行をお気に入りに追加"),
        Line::from("  ESC   : NORMAL に戻る"),
        Line::from(""),
        Line::from(Span::styled(
            "  [ESC] でキャンセル",
            base_style().fg(MONOKAI_GRAY),
        )),
    ];

    f.render_widget(
        Paragraph::new(help_lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ヘルプ (Keybinds) ")
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

#[cfg(test)]
#[path = "../tests/tui_ui.rs"]
mod tests;
