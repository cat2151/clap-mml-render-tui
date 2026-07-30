//! 起動待ちの進捗を画面中央へ出す overlay。
//!
//! 画面下部のステータス行だけだと、数秒の待ち時間に「固まったのか進んでいるのか」
//! が分からないため、視線の先（grid の中央）に2段階の進捗を重ねる。

use std::time::{Duration, Instant};

use ratatui::{
    style::Color,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{
        MONOKAI_CYAN, MONOKAI_DARK_GRAY, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PINK, MONOKAI_YELLOW,
    },
    ui::centered_text_block_rect,
};

use crate::{GridConnectionPhase, GridConnectionStatus, GridProgress};

const PREPARING_TITLE: &str = " Grid Sequencer 準備中 ";
const ERROR_TITLE: &str = " Grid Sequencer 準備エラー ";
/// バーの1目盛りは instance 1つ。grid の行と目で対応づけられるようにする。
const BAR_FILLED: &str = "█";
const BAR_EMPTY: &str = "░";
/// エラーメッセージ1行の桁数と、畳んだあとの最大行数。
const MESSAGE_WIDTH: usize = 60;
const MAX_MESSAGE_LINES: usize = 4;

pub(super) fn draw_overlay(
    f: &mut Frame<'_>,
    connection: &GridConnectionStatus,
    track_count: usize,
) {
    let (title, border_color, lines) = match connection.error_message() {
        Some(message) => (ERROR_TITLE, MONOKAI_PINK, error_lines(message)),
        None => (
            PREPARING_TITLE,
            MONOKAI_CYAN,
            progress_lines(connection, track_count),
        ),
    };
    let area = centered_text_block_rect(f.area(), title, &lines);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(base_style())
                .border_style(base_style().fg(border_color)),
        ),
        area,
    );
}

fn progress_lines(connection: &GridConnectionStatus, track_count: usize) -> Vec<Line<'static>> {
    let (startup, patches) = stage_progress(connection, track_count);
    let (startup_elapsed, patches_elapsed) = connection.stage_elapsed(Instant::now());
    let active = active_stage(connection);
    vec![
        Line::from(""),
        stage_line("1. サーバー起動", startup, startup_elapsed, active == 1),
        stage_line("2. 音色ロード  ", patches, patches_elapsed, active == 2),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}  ", footer_text(connection)),
            base_style().fg(MONOKAI_GRAY),
        )),
    ]
}

/// 2段階それぞれの進捗を `(サーバー起動, 音色ロード)` で返す。
///
/// patch ロードまで来ていればサーバー起動は必ず完了しているが、サーバーが既に
/// 起動済みだと `server_startup` が `None` のまま `PatchSetting` へ入るので、
/// phase から完了とみなす。
fn stage_progress(
    connection: &GridConnectionStatus,
    track_count: usize,
) -> (GridProgress, GridProgress) {
    let done = GridProgress {
        completed: track_count,
        total: track_count,
    };
    let none = GridProgress {
        completed: 0,
        total: track_count,
    };
    match &connection.phase {
        GridConnectionPhase::Connecting => (connection.server_startup.unwrap_or(none), none),
        GridConnectionPhase::WaitingForPatches => (done, none),
        GridConnectionPhase::PatchSetting => (done, connection.patch_setting.unwrap_or(none)),
        GridConnectionPhase::Ready => (done, done),
        GridConnectionPhase::Idle | GridConnectionPhase::Error(_) => (none, none),
    }
}

/// いま進んでいる段（1 / 2、どちらでもなければ 0）。
fn active_stage(connection: &GridConnectionStatus) -> u8 {
    match &connection.phase {
        GridConnectionPhase::Connecting => 1,
        GridConnectionPhase::WaitingForPatches | GridConnectionPhase::PatchSetting => 2,
        _ => 0,
    }
}

fn footer_text(connection: &GridConnectionStatus) -> &'static str {
    match &connection.phase {
        GridConnectionPhase::Connecting => "サーバーの起動を待っています",
        GridConnectionPhase::WaitingForPatches => "patch 一覧の読み込みを待っています",
        _ => "準備できた行から順に色が付きます",
    }
}

fn stage_line(
    label: &str,
    progress: GridProgress,
    elapsed: Option<Duration>,
    active: bool,
) -> Line<'static> {
    let style = base_style().fg(stage_color(progress, active));
    Line::from(vec![
        Span::styled(format!("  {label}  "), style),
        Span::styled(bar(progress), style),
        Span::styled(
            format!(" {:>2}/{}  ", progress.completed, progress.total),
            style,
        ),
        Span::styled(format!("{:>6}  ", format_elapsed(elapsed)), style),
    ])
}

/// 経過時間の表示。まだ始まっていない段は空欄にして、目が滑らないようにする。
fn format_elapsed(elapsed: Option<Duration>) -> String {
    elapsed.map_or_else(String::new, |elapsed| {
        format!("{:.1}s", elapsed.as_secs_f32())
    })
}

fn stage_color(progress: GridProgress, active: bool) -> Color {
    if progress.total > 0 && progress.completed >= progress.total {
        MONOKAI_GREEN
    } else if active {
        MONOKAI_YELLOW
    } else {
        MONOKAI_DARK_GRAY
    }
}

fn bar(progress: GridProgress) -> String {
    let total = progress.total.max(1);
    let filled = progress.completed.min(total);
    format!(
        "[{}{}]",
        BAR_FILLED.repeat(filled),
        BAR_EMPTY.repeat(total - filled)
    )
}

fn error_lines(message: &str) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from("")];
    lines.extend(wrap_message(message).into_iter().map(|text| {
        Line::from(Span::styled(
            format!("  {text}  "),
            base_style().fg(MONOKAI_PINK),
        ))
    }));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  r: 引き直して再試行   Ctrl+G: 画面切替  ",
        base_style().fg(MONOKAI_GRAY),
    )));
    lines
}

/// anyhow の文脈付きメッセージは長くなるので、固定幅で畳んで末尾を打ち切る。
fn wrap_message(message: &str) -> Vec<String> {
    let mut lines = message
        .chars()
        .collect::<Vec<_>>()
        .chunks(MESSAGE_WIDTH)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>();
    if lines.len() > MAX_MESSAGE_LINES {
        lines.truncate(MAX_MESSAGE_LINES);
        if let Some(last) = lines.last_mut() {
            last.pop();
            last.push('…');
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
