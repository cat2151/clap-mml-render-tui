//! 「音が鳴るまで」の待ちを段階で見せる中央 overlay（画面横断で共有）。
//!
//! ## なぜ要るか（実測）
//!
//! release ビルド・ユーザーの実キャッシュで、DAW を cold start したときの
//! `log.txt`（2026-09-03 19:30:38〜19:30:42）の内訳:
//!
//! | 段階 | 実測 |
//! |---|---|
//! | play server の起動（CLAP instance 14 本） | 1747ms（warm）／6559ms（cold） |
//! | 1 小節目のキャッシュ WAV ロード（7 track） | 2102ms（内訳: 1229 + 819 + 7〜11 × 5） |
//! | 合計（`play: start` → 最初の音） | 約 3.5〜4 秒。cold なら 10 秒近く |
//!
//! この数秒のあいだ、画面には**何も出ていなかった**。止まっているのか
//! 進んでいるのかが分からないので、段階と進み具合を中央へ出す。
//!
//! ## 描き方の流儀
//!
//! grid sequencer の history preview 進捗 overlay と同じ。
//! [`crate::ui::centered_text_block_rect`] で中身に合わせた枠を中央へ置き、
//! `Clear` で下地を消してから描く。**自前で新方式を作らないこと。**

use std::time::Duration;

use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::{
    status::base_style,
    theme::{MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_YELLOW},
    ui::centered_text_block_rect,
};

/// overlay の枠のタイトル。画面が違っても同じ待ちなので文言も揃える。
pub const STARTUP_PROGRESS_TITLE: &str = " 音が鳴るまで ";

/// 進捗バーの桁数。
const BAR_WIDTH: usize = 20;

/// 段階のラベル欄の桁数（日本語 1 文字 = 2 桁で数えた幅）。
const LABEL_WIDTH: usize = 26;

/// 1 段階の進み具合。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupStepState {
    /// まだ始まっていない。前の段階が終わるのを待っている。
    Waiting,
    /// 実行中。件数が分かるときだけ `(完了数, 総数)` を持つ。
    ///
    /// `None` になるのは「始まったが、まだ 1 件も報告が来ていない」とき。
    /// たとえば play server は子プロセスを spawn した直後で、
    /// `cmrt-server-startup: instances=N/M` がまだ 1 行も出ていない状態。
    Running(Option<(usize, usize)>),
    /// 終わった。
    Done,
}

/// overlay に並べる 1 段階。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupStep {
    pub label: String,
    pub state: StartupStepState,
}

impl StartupStep {
    pub fn new(label: impl Into<String>, state: StartupStepState) -> Self {
        Self {
            label: label.into(),
            state,
        }
    }
}

/// 段階と経過時間から overlay の本文を組む。
///
/// 描画と分けてあるのは、**テストが文字列だけを読めるようにする**ため。
/// 進み具合の見え方はここだけで決まる。
pub fn startup_progress_lines(steps: &[StartupStep], elapsed: Duration) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from("")];
    for step in steps {
        lines.push(step_line(step));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  経過 {:.1}s", elapsed.as_secs_f64()),
        base_style().fg(MONOKAI_GRAY),
    )));
    lines
}

fn step_line(step: &StartupStep) -> Line<'static> {
    let (marker, marker_color) = match step.state {
        StartupStepState::Waiting => ("  ", MONOKAI_GRAY),
        StartupStepState::Running(_) => ("▶ ", MONOKAI_YELLOW),
        StartupStepState::Done => ("✓ ", MONOKAI_GREEN),
    };
    let label_color = match step.state {
        StartupStepState::Waiting => MONOKAI_GRAY,
        StartupStepState::Running(_) => MONOKAI_FG,
        StartupStepState::Done => MONOKAI_GRAY,
    };
    Line::from(vec![
        Span::styled("  ", base_style()),
        Span::styled(marker.to_owned(), base_style().fg(marker_color)),
        Span::styled(pad_to_width(&step.label, LABEL_WIDTH), {
            let style = base_style().fg(label_color);
            if matches!(step.state, StartupStepState::Running(_)) {
                style.add_modifier(Modifier::BOLD)
            } else {
                style
            }
        }),
        Span::styled(progress_bar(step.state), base_style().fg(marker_color)),
        Span::styled(
            format!(" {}", progress_count(step.state)),
            base_style().fg(label_color),
        ),
    ])
}

/// 段階 1 つぶんのバー。**総数が分からない段階でも同じ桁数を返す**
/// （行の幅が段階ごとに変わると、枠の幅が毎フレーム揺れて読めなくなる）。
fn progress_bar(state: StartupStepState) -> String {
    let filled = match state {
        StartupStepState::Waiting | StartupStepState::Running(None) => 0,
        StartupStepState::Running(Some((completed, total))) => {
            if total == 0 {
                0
            } else {
                completed.min(total) * BAR_WIDTH / total
            }
        }
        StartupStepState::Done => BAR_WIDTH,
    };
    format!("{}{}", "█".repeat(filled), "░".repeat(BAR_WIDTH - filled))
}

fn progress_count(state: StartupStepState) -> String {
    match state {
        StartupStepState::Waiting => "-".to_string(),
        StartupStepState::Running(None) => "…".to_string(),
        StartupStepState::Running(Some((completed, total))) => format!("{completed}/{total}"),
        StartupStepState::Done => "done".to_string(),
    }
}

/// 端末上の桁数で右を埋める（日本語 1 文字が 2 桁ぶんを占めるため、
/// 文字数ではなく [`Span::width`] で数える）。
fn pad_to_width(text: &str, width: usize) -> String {
    let current = Span::raw(text.to_owned()).width();
    format!("{text}{}", " ".repeat(width.saturating_sub(current)))
}

/// 中央 overlay を描く。`area` は overlay を置きたい領域（普通は画面全体）。
pub fn draw_startup_progress_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    steps: &[StartupStep],
    elapsed: Duration,
) {
    let lines = startup_progress_lines(steps, elapsed);
    let overlay_area = centered_text_block_rect(area, STARTUP_PROGRESS_TITLE, &lines);
    frame.render_widget(Clear, overlay_area);
    frame.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(STARTUP_PROGRESS_TITLE)
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        overlay_area,
    );
}

#[cfg(test)]
mod tests;
