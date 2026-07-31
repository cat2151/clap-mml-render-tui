use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_DARK_GRAY, MONOKAI_GRAY, MONOKAI_GREEN},
};

use crate::{
    ChordPlayback, GridConnectionStatus, GridRow, GridRowReadiness, GridSequencerScreen,
    StepDuration, CHORD_ROW, GRID_STEPS,
};

/// patch name 欄の桁数。
const PATCH_WIDTH: usize = 24;
/// note on のセル記号と、休符のセル記号（1セルは記号+空白の2桁）。
const NOTE_CELL: &str = "# ";
const REST_CELL: &str = ". ";

pub(super) fn draw(
    screen: &GridSequencerScreen,
    connection: &GridConnectionStatus,
    f: &mut Frame<'_>,
    area: Rect,
) {
    let playhead = screen.state.step_index();
    let chord = screen.state.chord();
    let mut lines = vec![header_line()];
    lines.extend(screen.state.rows().iter().enumerate().map(|(index, row)| {
        // 和音の行だけ、セルではなく現在のコードを表示する。
        let chord = chord.filter(|_| index == CHORD_ROW);
        row_line(index, row, chord, playhead, connection.row_readiness(index))
    }));
    f.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Grid Sequencer ")
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

fn header_line() -> Line<'static> {
    let style = base_style().fg(MONOKAI_GRAY);
    Line::from(vec![
        Span::styled(label_columns("#", "PATCH", "NOTE", "DUR"), style),
        Span::styled(step_ruler(), style),
    ])
}

fn row_line(
    index: usize,
    row: &GridRow,
    chord: Option<&ChordPlayback>,
    playhead: usize,
    readiness: GridRowReadiness,
) -> Line<'static> {
    let (note, duration) = match chord {
        // chord mode の行はセルも音長も使わず、全音符で和音を鳴らす。
        Some(chord) => (
            chord.current().first().copied().unwrap_or(row.note),
            StepDuration::Whole,
        ),
        None => (row.note, row.duration),
    };
    let mut spans = vec![Span::styled(
        label_columns(
            &(index + 1).to_string(),
            &truncate_patch(row.patch.as_deref(), PATCH_WIDTH),
            &note.to_string(),
            duration.label(),
        ),
        label_style(readiness),
    )];
    for step in 0..GRID_STEPS {
        // 和音は先頭ステップだけ発音する。セルの設定値ではなくそれを見せる。
        let on = match chord {
            Some(_) => step == 0,
            None => row.cells[step],
        };
        let style = cell_style(readiness, on);
        let style = if step == playhead {
            cursor_highlight_style(style)
        } else {
            style
        };
        spans.push(Span::styled(if on { NOTE_CELL } else { REST_CELL }, style));
    }
    Line::from(spans)
}

/// 準備の済んでいない行は左の情報欄も落として、鳴らせる行と見分けられるようにする。
fn label_style(readiness: GridRowReadiness) -> Style {
    match readiness {
        GridRowReadiness::Prepared => base_style(),
        GridRowReadiness::InstanceReady => base_style().fg(MONOKAI_GRAY),
        GridRowReadiness::Pending => base_style().fg(MONOKAI_DARK_GRAY),
    }
}

/// 準備中の行は note / 休符を同じ色に落とす。パターン自体は記号（`#` / `.`）で読める。
fn cell_style(readiness: GridRowReadiness, on: bool) -> Style {
    let color = match readiness {
        GridRowReadiness::Prepared if on => MONOKAI_GREEN,
        GridRowReadiness::Prepared | GridRowReadiness::InstanceReady => MONOKAI_GRAY,
        GridRowReadiness::Pending => MONOKAI_DARK_GRAY,
    };
    base_style().fg(color)
}

/// grid の左に置く情報欄（行番号 / patch name / note number / 音長）。
fn label_columns(index: &str, patch: &str, note: &str, duration: &str) -> String {
    format!(
        " {index:>2} {patch:<width$} {note:>4} {duration:>5} ",
        width = PATCH_WIDTH
    )
}

/// 4ステップごとに1始まりの番号を出す目盛り。
fn step_ruler() -> String {
    (0..GRID_STEPS)
        .map(|step| {
            if step % 4 == 0 {
                format!("{:<2}", step + 1)
            } else {
                "  ".to_string()
            }
        })
        .collect()
}

/// 長い patch name は先頭を省略する（末尾のファイル名のほうが判別に役立つため）。
fn truncate_patch(patch: Option<&str>, width: usize) -> String {
    let Some(patch) = patch else {
        return "-".to_string();
    };
    let chars = patch.chars().collect::<Vec<_>>();
    if chars.len() <= width {
        return patch.to_string();
    }
    let tail = chars[chars.len() + 1 - width..].iter().collect::<String>();
    format!("…{tail}")
}
