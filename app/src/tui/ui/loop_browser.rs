use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::status::{base_style, loop_browser_keybind_text, status_color, status_text};
use crate::tui::loop_browser::PAD_KEYS;
use crate::tui::{Mode, TuiApp};
use crate::ui_theme::{MONOKAI_CYAN, MONOKAI_FG, MONOKAI_YELLOW};

mod help;
mod tracks;
mod tree;
mod waveforms;

pub(super) fn draw(app: &mut TuiApp<'_>, frame: &mut Frame) {
    let draw_started = std::time::Instant::now();
    let trace_id = app.loop_browser.state.pending_render_trace.take();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(frame.area());
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let tree_started = std::time::Instant::now();
    let rendered_tree_nodes = tree::draw(app, frame, panes[0]);
    let tree_elapsed = tree_started.elapsed();
    let tracks_started = std::time::Instant::now();
    tracks::draw(app, frame, panes[1]);
    let tracks_elapsed = tracks_started.elapsed();
    let pads_started = std::time::Instant::now();
    draw_pads(app, frame, chunks[1]);
    let pads_elapsed = pads_started.elapsed();

    let play_state = app.play_state.lock().unwrap().clone();
    let persistence_error = app
        .loop_browser
        .state
        .metadata_error
        .as_ref()
        .or(app.loop_browser.state.track_grid_error.as_ref())
        .or(app.loop_browser.state.random_decks_error.as_ref());
    let (status, color) = if let Some(error) = persistence_error {
        (error.clone(), Color::Red)
    } else if matches!(play_state, crate::tui::PlayState::Err(_)) {
        (
            status_text(&Mode::LoopBrowser, &play_state),
            status_color(&play_state),
        )
    } else if app.loop_browser.state.playback_paused {
        ("loop browser  ⏸ 停止中".to_string(), MONOKAI_CYAN)
    } else {
        (
            status_text(&Mode::LoopBrowser, &play_state),
            status_color(&play_state),
        )
    };
    frame.render_widget(
        Paragraph::new(status).style(base_style().fg(color)),
        chunks[2],
    );
    frame.render_widget(
        Paragraph::new(loop_browser_keybind_text(app.loop_browser.state.focus)).style(base_style()),
        chunks[3],
    );

    if app.loop_browser.state.category_overlay.is_some() {
        draw_category_overlay(app, frame);
    }
    if app.loop_browser.state.mixer_overlay_open {
        draw_mixer_overlay(app, frame);
    }
    if let Some(notice) = app
        .loop_browser
        .state
        .active_notice()
        .map(|notice| notice.text.clone())
    {
        draw_notice(frame, &notice);
    }
    if let Some(pane) = app.loop_browser.state.help_overlay {
        help::draw(frame, pane);
    }
    if app.loop_browser.state.starting {
        draw_startup_overlay(frame);
    }
    app.loop_browser.state.last_render_metrics =
        Some(crate::tui::loop_browser::performance::RenderMetrics {
            trace_id,
            tree: tree_elapsed,
            tracks: tracks_elapsed,
            pads: pads_elapsed,
            draw: draw_started.elapsed(),
            rendered_tree_nodes,
            total_tree_nodes: app.loop_browser.state.visible.len(),
        });
}

fn draw_startup_overlay(frame: &mut Frame<'_>) {
    let lines = vec![Line::from("Loop Browser 起動中…")];
    let area = crate::ui_utils::centered_text_block_rect(frame.area(), " Loop Browser ", &lines);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Loop Browser ")
                    .border_style(base_style().fg(MONOKAI_CYAN)),
            ),
        area,
    );
}

fn draw_mixer_overlay(app: &TuiApp<'_>, frame: &mut Frame<'_>) {
    let area = frame.area();
    let tracks = app
        .loop_browser
        .state
        .track_grid()
        .iter()
        .enumerate()
        .map(|(track, _)| crate::mixer_overlay::MixerOverlayTrack {
            label: format!("track{}", track + 1),
            volume_db: app.loop_browser.state.track_volume_db(track),
        })
        .collect::<Vec<_>>();
    crate::mixer_overlay::draw_mixer_overlay(
        frame,
        area,
        &tracks,
        app.loop_browser.state.mixer_cursor_track,
    );
}

fn draw_pads(app: &TuiApp<'_>, frame: &mut Frame<'_>, area: Rect) {
    let spans = PAD_KEYS
        .iter()
        .flat_map(|pad| {
            let name = app
                .loop_browser
                .state
                .pad_file_name(*pad)
                .unwrap_or_else(|| "-".to_string());
            [
                Span::styled(
                    format!(" {}:", pad.to_ascii_uppercase()),
                    base_style().fg(MONOKAI_YELLOW),
                ),
                Span::raw(format!("{name} ")),
            ]
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" [WAV PADS] ")
                    .border_style(base_style().fg(MONOKAI_CYAN)),
            ),
        area,
    );
}

fn focus_border_style(focused: bool) -> Style {
    if focused {
        base_style().fg(MONOKAI_CYAN).add_modifier(Modifier::BOLD)
    } else {
        base_style().fg(MONOKAI_FG)
    }
}

fn draw_category_overlay(app: &TuiApp<'_>, frame: &mut Frame<'_>) {
    let current = app.loop_browser.state.category_overlay_current();
    let lines = app
        .loop_browser
        .state
        .category_keys
        .iter()
        .map(|(key, category)| {
            let marker = if current == Some(category.as_str()) {
                "●"
            } else {
                " "
            };
            Line::from(format!(" {marker} {key}: {category} "))
        })
        .collect::<Vec<_>>();
    let area = crate::ui_utils::centered_text_block_rect(
        frame.area(),
        " dirカテゴリ (Esc:キャンセル) ",
        &lines,
    );
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" dirカテゴリ (Esc:キャンセル) ")
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

fn draw_notice(frame: &mut Frame<'_>, message: &str) {
    let lines = vec![Line::from(Span::styled(
        message.to_string(),
        base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
    ))];
    let area = crate::ui_utils::centered_text_block_rect(frame.area(), " お知らせ ", &lines);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" お知らせ ")
                    .border_style(base_style().fg(MONOKAI_CYAN)),
            ),
        area,
    );
}
