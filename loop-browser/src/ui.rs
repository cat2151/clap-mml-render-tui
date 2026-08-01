use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::{loop_browser_keybind_text, LoopBrowser, PAD_KEYS};
use cmrt_tui_core::status::{base_style, play_status_suffix, status_color};
use cmrt_tui_core::theme::{MONOKAI_CYAN, MONOKAI_FG, MONOKAI_YELLOW};
use cmrt_tui_core::PlayState;

mod help;
mod tracks;
mod tree;
mod waveforms;

pub fn draw(state: &mut LoopBrowser, play_state: &PlayState, frame: &mut Frame) {
    let draw_started = std::time::Instant::now();
    let trace_id = state.pending_render_trace.take();
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
    let rendered_tree_nodes = tree::draw(state, frame, panes[0]);
    let tree_elapsed = tree_started.elapsed();
    let tracks_started = std::time::Instant::now();
    tracks::draw(state, frame, panes[1]);
    let tracks_elapsed = tracks_started.elapsed();
    let pads_started = std::time::Instant::now();
    draw_pads(state, frame, chunks[1]);
    let pads_elapsed = pads_started.elapsed();

    let persistence_error = state.persistence_error();
    let (status, color) = if let Some(error) = persistence_error {
        (error.clone(), Color::Red)
    } else if matches!(play_state, PlayState::Err(_)) {
        (
            format!("loop browser{}", play_status_suffix(play_state)),
            status_color(play_state),
        )
    } else if state.playback_paused {
        (
            format!("loop browser{}  ⏸ 停止中", auto_random_suffix(state)),
            MONOKAI_CYAN,
        )
    } else {
        (
            format!(
                "loop browser{}{}",
                auto_random_suffix(state),
                play_status_suffix(play_state)
            ),
            status_color(play_state),
        )
    };
    frame.render_widget(
        Paragraph::new(status).style(base_style().fg(color)),
        chunks[2],
    );
    frame.render_widget(
        Paragraph::new(loop_browser_keybind_text(state.focus)).style(base_style()),
        chunks[3],
    );

    if state.category_overlay.is_some() {
        draw_category_overlay(state, frame);
    }
    if state.mixer_overlay_open {
        draw_mixer_overlay(state, frame);
    }
    if let Some(notice) = state.active_notice().map(|notice| notice.text.clone()) {
        draw_notice(frame, &notice);
    }
    if let Some(pane) = state.help_overlay {
        help::draw(frame, pane);
    }
    if state.starting {
        draw_startup_overlay(frame);
    }
    state.last_render_metrics = Some(crate::performance::RenderMetrics {
        trace_id,
        tree: tree_elapsed,
        tracks: tracks_elapsed,
        pads: pads_elapsed,
        draw: draw_started.elapsed(),
        rendered_tree_nodes,
        total_tree_nodes: state.visible.len(),
    });
}

fn draw_startup_overlay(frame: &mut Frame<'_>) {
    let lines = vec![Line::from("Loop Browser 起動中…")];
    let area = cmrt_tui_core::ui::centered_text_block_rect(frame.area(), " Loop Browser ", &lines);
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

fn draw_mixer_overlay(state: &LoopBrowser, frame: &mut Frame<'_>) {
    let area = frame.area();
    let tracks = state
        .track_grid()
        .iter()
        .enumerate()
        .map(
            |(track, _)| cmrt_tui_core::mixer_overlay::MixerOverlayTrack {
                label: format!("track{}", track + 1),
                volume_db: state.track_volume_db(track),
            },
        )
        .collect::<Vec<_>>();
    cmrt_tui_core::mixer_overlay::draw_mixer_overlay(
        frame,
        area,
        &tracks,
        state.mixer_cursor_track,
    );
}

fn draw_pads(state: &LoopBrowser, frame: &mut Frame<'_>, area: Rect) {
    let spans = PAD_KEYS
        .iter()
        .flat_map(|pad| {
            let name = state.pad_file_name(*pad).unwrap_or_else(|| "-".to_string());
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

fn draw_category_overlay(state: &LoopBrowser, frame: &mut Frame<'_>) {
    let current = state.category_overlay_current();
    let lines = state
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
    let area = cmrt_tui_core::ui::centered_text_block_rect(
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

/// オートランダムモードが ON のときだけステータス行に出すマーク。
fn auto_random_suffix(state: &LoopBrowser) -> &'static str {
    if state.auto_random() {
        "  [AUTO 2周]"
    } else {
        ""
    }
}

fn draw_notice(frame: &mut Frame<'_>, message: &str) {
    let lines = vec![Line::from(Span::styled(
        message.to_string(),
        base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
    ))];
    let area = cmrt_tui_core::ui::centered_text_block_rect(frame.area(), " お知らせ ", &lines);
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
