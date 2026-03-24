//! TUI 描画

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use super::{Mode, PatchPhrasePane, PlayState, TuiApp};

pub(super) fn draw(app: &mut TuiApp<'_>, f: &mut Frame) {
    // play_state を一度だけロックしてスナップショットを取り、
    // status_text と status_color を同じ状態から導出する（二重ロック・状態不整合を防ぐ）。
    let play_state = app.play_state.lock().unwrap().clone();
    let status = status_text(app, &play_state);
    let status_color = status_color(&play_state);

    if app.mode == Mode::Help {
        draw_normal(app, f, &status, status_color);
        draw_help(f);
    } else if app.mode == Mode::PatchSelect {
        draw_patch_select(app, f, &status, status_color);
    } else if app.mode == Mode::PatchPhrase {
        draw_patch_phrase(app, f, &status, status_color);
    } else {
        draw_normal(app, f, &status, status_color);
    }
}

fn status_color(play_state: &PlayState) -> Color {
    match play_state {
        PlayState::Err(_) => Color::Red,
        PlayState::Running(_) => Color::Magenta,
        PlayState::Playing(_) => Color::Yellow,
        PlayState::Done(_) => Color::Green,
        PlayState::Idle => Color::Cyan,
    }
}

fn status_text(app: &TuiApp<'_>, play_state: &PlayState) -> String {
    let play_str = match play_state {
        PlayState::Idle => "".to_string(),
        PlayState::Running(mml) => format!("  ⚙ レンダリング中: {}", mml),
        PlayState::Playing(msg) => format!("  ▶ 演奏中: {}", msg),
        PlayState::Done(msg) => format!("  ✓ {}", msg),
        PlayState::Err(msg) => format!("  ✗ {}", msg),
    };
    match app.mode {
        Mode::Normal => format!(
            "NORMAL  i:INS  r:ランダム音色  t:音色  p:phrase  j/k  Enter  d:DAW  K:Help  q{}",
            play_str
        ),
        Mode::Insert => format!("ESC:確定→NORMAL  Enter:確定→次行{}", play_str),
        Mode::PatchSelect => format!(
            "音色選択  Enter:決定  ESC:キャンセル  ↑↓:移動  文字入力:フィルタ  Space:AND条件{}",
            play_str
        ),
        Mode::PatchPhrase => {
            format!(
                "patch phrase  j/k:再生移動  h/l:ペイン移動  f:お気に入り  ESC:戻る{}",
                play_str
            )
        }
        Mode::Help => format!("HELP  ESC:キャンセル{}", play_str),
    }
}

fn draw_patch_select(app: &mut TuiApp<'_>, f: &mut Frame, status: &str, status_color: Color) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(f.area());

    f.render_widget(
        Paragraph::new(format!("> {}", app.patch_query)).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 音色選択 - 検索 (space=AND) ")
                .border_style(Style::default().fg(Color::Yellow)),
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
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(p.clone(), style))
        })
        .collect();

    f.render_stateful_widget(
        List::new(patch_items)
            .block(Block::default().borders(Borders::ALL).title(count_title))
            .highlight_symbol("▶ "),
        chunks[1],
        &mut app.patch_list_state,
    );

    f.render_widget(
        Paragraph::new(status.to_string()).style(Style::default().fg(status_color)),
        chunks[2],
    );
}

fn draw_normal(app: &mut TuiApp<'_>, f: &mut Frame, status: &str, status_color: Color) {
    let is_insert = app.mode == Mode::Insert;
    let cursor = app.cursor;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(f.area());

    // キャッシュのガードを保持したままイテレートすることで、全キーのクローンを避ける。
    let cache_guard = app.audio_cache.lock().unwrap();
    let items: Vec<ListItem> = app
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let mml = line.trim();
            let is_cached = !mml.is_empty() && cache_guard.contains_key(mml);
            let style = if i == cursor {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_cached {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:>3} ", i + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(line.clone(), style),
            ]))
        })
        .collect();

    f.render_stateful_widget(
        List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" MML Lines "))
            .highlight_symbol("▶ "),
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
                    x: list_area.x + 1,
                    y: textarea_y,
                    width: list_area.width.saturating_sub(2),
                    height: 1,
                };
                f.render_widget(&app.textarea, textarea_area);
            }
        }
    }

    f.render_widget(
        Paragraph::new(status.to_string()).style(Style::default().fg(status_color)),
        chunks[1],
    );
}

fn draw_patch_phrase(app: &mut TuiApp<'_>, f: &mut Frame, status: &str, status_color: Color) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
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
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
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
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(phrase, style))
        })
        .collect();

    let history_border = if app.patch_phrase_focus == PatchPhrasePane::History {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };
    let favorites_border = if app.patch_phrase_focus == PatchPhrasePane::Favorites {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    f.render_stateful_widget(
        List::new(history_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" History - {patch_name} "))
                    .border_style(history_border),
            )
            .highlight_symbol("▶ "),
        panes[0],
        &mut app.patch_phrase_history_state,
    );
    f.render_stateful_widget(
        List::new(favorite_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Favorites ")
                    .border_style(favorites_border),
            )
            .highlight_symbol("▶ "),
        panes[1],
        &mut app.patch_phrase_favorites_state,
    );

    f.render_widget(
        Paragraph::new(status.to_string()).style(Style::default().fg(status_color)),
        chunks[1],
    );
}

fn draw_help(f: &mut Frame) {
    let area = crate::ui_utils::centered_rect(60, 80, f.area());
    f.render_widget(Clear, area);

    let help_lines = vec![
        Line::from(Span::styled(
            "NORMAL モード",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  j / ↓       : 下へ移動"),
        Line::from("  k / ↑       : 上へ移動"),
        Line::from("  H           : 先頭行へ移動"),
        Line::from("  M           : 中央行へ移動"),
        Line::from("  L           : 末尾行へ移動"),
        Line::from("  Enter/Space : 再生"),
        Line::from("  i           : INSERT モード"),
        Line::from("  o           : 次行に挿入 → INSERT"),
        Line::from("  r           : 現在行の先頭にランダム音色を挿入/置換"),
        Line::from("  t           : 音色選択"),
        Line::from("  p           : patch phrase 画面"),
        Line::from("  d           : DAW モード"),
        Line::from("  K           : ヘルプ (このページ)"),
        Line::from("  q           : 終了"),
        Line::from("  Ctrl+C      : 強制終了"),
        Line::from(""),
        Line::from(Span::styled(
            "INSERT モード",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  ESC   : 確定 → NORMAL (再生)"),
        Line::from("  Enter : 確定 → 次行挿入 → INSERT 継続"),
        Line::from(""),
        Line::from(Span::styled(
            "音色選択モード",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  文字入力 : フィルタ (Space=AND条件)"),
        Line::from("  ↑↓      : リスト移動"),
        Line::from("  Enter   : 音色決定"),
        Line::from("  ESC     : キャンセル"),
        Line::from(""),
        Line::from(Span::styled(
            "patch phrase 画面",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  j / k : 上下移動して再生"),
        Line::from("  h / l : ペイン切替して再生"),
        Line::from("  f     : 現在行をお気に入りに追加"),
        Line::from("  ESC   : NORMAL に戻る"),
        Line::from(""),
        Line::from(Span::styled(
            "  [ESC] でキャンセル",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(
        Paragraph::new(help_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ヘルプ (Keybinds) ")
                .border_style(Style::default().fg(Color::Cyan)),
        ),
        area,
    );
}

#[cfg(test)]
mod tests {
    use ratatui::{backend::TestBackend, Terminal};

    use crate::{config::Config, history::PatchPhraseState, tui::TuiApp};

    use super::{draw, Mode};

    fn test_config() -> Config {
        Config {
            plugin_path: "/tmp/Surge XT.clap".to_string(),
            input_midi: "input.mid".to_string(),
            output_midi: "output.mid".to_string(),
            output_wav: "output.wav".to_string(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patch_path: None,
            patches_dir: Some("/tmp/patches".to_string()),
            daw_tracks: 9,
            daw_measures: 8,
        }
    }

    fn render_lines(app: &mut TuiApp<'static>, width: u16, height: u16) -> Vec<String> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(app, f)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn patch_phrase_screen_renders_history_and_favorites_lists() {
        let mut app = TuiApp::new_for_test(test_config());
        app.mode = Mode::PatchPhrase;
        app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
        app.patch_phrase_store.patches.insert(
            "Pads/Pad 1.fxp".to_string(),
            PatchPhraseState {
                history: vec!["l8cdef".to_string()],
                favorites: vec!["o5g".to_string()],
            },
        );
        app.patch_phrase_history_state.select(Some(0));
        app.patch_phrase_favorites_state.select(Some(0));

        let lines = render_lines(&mut app, 80, 10).join("\n");

        assert!(lines.contains("History - Pads/Pad 1.fxp"));
        assert!(lines.contains("Favorites"));
        assert!(lines.contains("l8cdef"));
        assert!(lines.contains("o5g"));
    }

    #[test]
    fn patch_phrase_screen_uses_c_as_fallback_for_empty_lists() {
        let mut app = TuiApp::new_for_test(test_config());
        app.mode = Mode::PatchPhrase;
        app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
        app.patch_phrase_history_state.select(Some(0));
        app.patch_phrase_favorites_state.select(Some(0));

        let lines = render_lines(&mut app, 80, 10).join("\n");

        assert!(lines.contains("▶ c"));
    }
}
